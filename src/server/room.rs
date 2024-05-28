use std::{collections::HashMap, sync::Arc};

use circular_buffer::CircularBuffer;
use log::info;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;

use crate::{
    server::{
        command::{Command, CommandParseError},
        user,
    },
    utils,
};

use super::{
    app::{self, MessageChannel},
    message::{self, Message},
    motd::Motd,
    state::UserState,
    terminal::TerminalHandle,
    user::User,
};

const MESSAGE_HISTORY_LEN: usize = 20;

type UserId = usize;
type UserName = String;
type MessageHistory = CircularBuffer<MESSAGE_HISTORY_LEN, Message>;

#[derive(Clone)]
pub struct ServerRoom {
    pub names: HashMap<UserId, UserName>,
    members: HashMap<UserName, app::App>,
    motd: Motd,
    history: MessageHistory,
}

impl ServerRoom {
    pub fn new() -> Self {
        Self {
            names: HashMap::new(),
            members: HashMap::new(),
            motd: Default::default(),
            history: MessageHistory::new(),
        }
    }

    pub fn members_mut(&mut self) -> &mut HashMap<UserName, app::App> {
        &mut self.members
    }

    pub fn motd(&self) -> &String {
        &self.motd.get()
    }

    pub async fn join(
        &mut self,
        user_id: UserId,
        username: UserName,
        fingerpint: String,
        terminal: TerminalHandle,
        ssh_id: &[u8],
    ) {
        info!("join {}", user_id);
        let name = match self.is_member(&username).await {
            true => User::gen_rand_name(),
            false => username,
        };

        let user = User::new(
            user_id,
            name.clone(),
            String::from_utf8_lossy(ssh_id).to_string(),
            fingerpint,
        );

        let member = app::App {
            user: user.clone(),
            state: UserState::new(),
            terminal: Arc::new(Mutex::new(terminal)),
            channel: MessageChannel::new(),
        };

        self.members.insert(name.clone(), member.clone());
        self.names.insert(user_id, name.clone());

        self.feed_history(&name).await;

        let join_msg_body = format!("joined. (Connected: {})", self.members.len());
        self.send_message(message::Announce::new(user, join_msg_body).into())
            .await;
    }

    pub async fn feed_history(&mut self, username: &UserName) {
        let history = self.history.clone();
        let app = self.members.get(username).unwrap();
        for msg in history.iter() {
            if let Err(_) = app.send_message(msg.to_owned()).await {
                continue;
            }
            info!("Message sent!");
        }
    }

    pub async fn send_message(&mut self, msg: Message) {
        match msg {
            Message::System(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg).await.unwrap();
            }
            Message::Command(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg).await.unwrap();
            }
            Message::Error(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg).await.unwrap();
            }
            Message::Public(_) => {
                self.history.push_back(msg.clone());
                for (_, member) in self.members.iter() {
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Emote(_) => {
                self.history.push_back(msg.clone());
                for (_, member) in self.members.iter() {
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Announce(_) => {
                self.history.push_back(msg.clone());
                for (_, member) in self.members.iter() {
                    if member.user.quiet {
                        continue;
                    }
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Private(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg.clone()).await.unwrap();

                let to = self.members.get(&m.to.username).unwrap();
                to.send_message(msg).await.unwrap();
            }
        }
    }

    pub async fn handle_input(&mut self, user_id: &UserId, data: &[u8]) {
        let mut username = self.names.get(user_id).unwrap().clone();

        let mut decoder = Decoder::new();
        for keycode in decoder.write(data[0]) {
            match keycode {
                KeyCode::Enter => {
                    let cmd = {
                        let member = self.members.get_mut(&username).unwrap();
                        Command::parse(&member.state.input.bytes())
                    };

                    match cmd {
                        Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                            let message = {
                                let member = self.members.get_mut(&username).unwrap();
                                message::Public::new(
                                    member.user.clone(),
                                    member.state.input.to_str(),
                                )
                                .into()
                            };
                            self.send_message(message).await;

                            let member = self.members.get_mut(&username).unwrap();
                            member.state.input.clear();

                            return;
                        }
                        Err(err) => {
                            let message = {
                                let member = self.members.get_mut(&username).unwrap();
                                let mut input_iter =
                                    std::str::from_utf8(&member.state.input.bytes())
                                        .expect("Input must be a valid UTF-8 string")
                                        .split_whitespace()
                                        .into_iter();
                                message::Command::new(
                                    member.user.clone(),
                                    input_iter.nth(0).unwrap().to_string(),
                                    input_iter.collect::<Vec<_>>().join(" "),
                                )
                                .into()
                            };
                            self.send_message(message).await;

                            let message = {
                                let member = self.members.get_mut(&username).unwrap();
                                message::Error::new(member.user.clone(), format!("{}", err)).into()
                            };
                            self.send_message(message).await;

                            let member = self.members.get_mut(&username).unwrap();
                            member.state.input.clear();

                            return;
                        }
                        Ok(_) => {
                            let message = {
                                let member = self.members.get_mut(&username).unwrap();
                                let mut input_iter =
                                    std::str::from_utf8(&member.state.input.bytes())
                                        .expect("Input must be a valid UTF-8 string")
                                        .split_whitespace()
                                        .into_iter();
                                message::Command::new(
                                    member.user.clone(),
                                    input_iter.nth(0).unwrap().to_string(),
                                    input_iter.collect::<Vec<_>>().join(" "),
                                )
                                .into()
                            };
                            self.send_message(message).await;
                        }
                    }

                    match cmd.unwrap() {
                        Command::Exit => {
                            let app = self.members.get_mut(&username).unwrap().clone();

                            let duration = humantime::format_duration(app.user.joined_duration());
                            let message = message::Announce::new(
                                app.user.clone(),
                                format!("left: (After {})", duration),
                            );
                            self.send_message(message.into()).await;

                            self.members.remove(&username);
                            self.names.remove(&user_id);
                            return;
                        }
                        Command::Away(reason) => {
                            let from = self.members.get_mut(&username).unwrap();
                            from.user.go_away(reason.to_string());

                            let message = message::Emote::new(
                                from.user.clone(),
                                format!("has gone away: \"{}\"", reason),
                            );
                            self.send_message(message.into()).await;
                        }
                        Command::Back => {
                            let from = self.members.get_mut(&username).unwrap();
                            match &from.user.status {
                                user::UserStatus::Active => {}
                                user::UserStatus::Away {
                                    reason: _,
                                    since: _,
                                } => {
                                    from.user.return_active();
                                    let message = message::Emote::new(
                                        from.user.clone(),
                                        "is back".to_string(),
                                    );
                                    self.send_message(message.into()).await;
                                }
                            }
                        }
                        Command::Name(new_name) => 'label: {
                            let from = self.members.get_mut(&username).unwrap();
                            let user = from.user.clone();

                            if user.username == new_name {
                                let message = message::Error::new(
                                    user.clone(),
                                    "New name is the same as the original".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            if let Some(_) = self.members.get(&new_name) {
                                let message = message::Error::new(
                                    user.clone(),
                                    format!("\"{}\" name is already taken", new_name),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let message = message::Announce::new(
                                user.clone(),
                                format!("user is now known as {}.", new_name),
                            );
                            self.send_message(message.into()).await;

                            let new_name = new_name.to_string();
                            let old_name = user.username;
                            let user_id = user.id;

                            let from = self.members.get_mut(&username).unwrap();
                            from.user.set_new_name(new_name.clone());

                            let app = from.clone();
                            self.members.insert(new_name.clone(), app);
                            self.members.remove(&old_name);
                            self.names.insert(user_id, new_name.clone());
                            username = new_name
                        }
                        Command::Msg(to, msg) => {
                            let from = self.members.get_mut(&username).unwrap().clone();

                            match self.members.get(&to) {
                                Some(member) if from.user.id.eq(&member.user.id) => {
                                    self.send_message(
                                        message::Error::new(
                                            from.user.clone(),
                                            format!("You can't message yourself"),
                                        )
                                        .into(),
                                    )
                                    .await;
                                }
                                Some(member) => {
                                    let target_status = member.user.status.clone();
                                    let target_name = member.user.username.clone();

                                    self.send_message(
                                        message::Private::new(
                                            from.user.clone(),
                                            member.user.clone(),
                                            msg.to_string(),
                                        )
                                        .into(),
                                    )
                                    .await;

                                    match target_status {
                                        user::UserStatus::Away { reason, since: _ } => {
                                            self.send_message(
                                                message::System::new(
                                                    from.user.clone(),
                                                    format!(
                                                        "Sent PM to {}, but they're away now: {}",
                                                        target_name, reason
                                                    ),
                                                )
                                                .into(),
                                            )
                                            .await;
                                        }
                                        user::UserStatus::Active => {}
                                    }
                                }
                                None => {
                                    self.send_message(
                                        message::Error::new(
                                            from.user.clone(),
                                            format!("User is not found"),
                                        )
                                        .into(),
                                    )
                                    .await;
                                }
                            }

                            if let Some(to) = self.members.get_mut(&to) {
                                if !from.user.id.eq(&to.user.id) {
                                    to.user.set_reply_to(from.user.id);
                                }
                            }
                        }
                        Command::Reply(body) => 'label: {
                            let from = self.members.get(&username).unwrap().clone();
                            if from.user.reply_to.is_none() {
                                let message = message::Error::new(
                                    from.user.clone(),
                                    "There is no message to reply to".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let target_id = &from.user.reply_to.unwrap();
                            let target_name = self.names.get(&target_id);
                            if target_name.is_none() {
                                let message = message::Error::new(
                                    from.user.clone(),
                                    "User already left the room".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let to = self.members.get(target_name.unwrap()).unwrap().clone();
                            let message =
                                message::Private::new(from.user.clone(), to.user.clone(), body);
                            self.send_message(message.into()).await;
                        }
                        Command::Users => {
                            let from = self.members.get(&username).unwrap().clone();
                            let mut usernames = self.names.values().collect::<Vec<&String>>();
                            usernames.sort_by_key(|a| a.to_lowercase());

                            let colorized_names = usernames
                                .iter()
                                .map(|u| from.user.theme.style_username(u).to_string())
                                .collect::<Vec<String>>();

                            let body = format!(
                                "{} connected: {}",
                                self.names.len(),
                                colorized_names.join(", ")
                            );

                            self.send_message(message::System::new(from.user.clone(), body).into())
                                .await;
                        }
                        Command::Whois(target) => {
                            let from = self.members.get(&username).unwrap().clone();
                            let message = match self.members.get(&target) {
                                Some(member) => {
                                    message::System::new(from.user.clone(), member.user.to_string())
                                        .into()
                                }
                                None => message::Error::new(
                                    from.user.clone(),
                                    "User is not found".to_string(),
                                )
                                .into(),
                            };
                            self.send_message(message).await;
                        }
                        Command::Slap(target) => 'label: {
                            let from = self.members.get_mut(&username).unwrap().clone();
                            if target.is_none() {
                                let message = message::Emote::new(
                                    from.user.clone(),
                                    "hits himself with a squishy banana.".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let target = target.unwrap();
                            let target = self.members.get_mut(&target).map(|app| &mut app.user);

                            let message = if let Some(u) = target {
                                message::Emote::new(
                                    from.user.clone(),
                                    format!("hits {} with a squishy banana.", u.username),
                                )
                                .into()
                            } else {
                                message::Error::new(
                                    from.user.clone(),
                                    "That slippin' monkey is not in the room".to_string(),
                                )
                                .into()
                            };
                            self.send_message(message).await;
                        }
                        Command::Shrug => {
                            let from = self.members.get_mut(&username).unwrap().clone();
                            self.send_message(
                                message::Emote::new(from.user.clone(), "¯\\_(ツ)_/¯".to_string())
                                    .into(),
                            )
                            .await;
                        }
                        Command::Me(action) => {
                            let from = self.members.get_mut(&username).unwrap().clone();
                            let message = message::Emote::new(
                                from.user.clone(),
                                match action {
                                    Some(s) => format!("{}", s),
                                    None => format!("is at a loss for words."),
                                },
                            );
                            self.send_message(message.into()).await;
                        }
                        Command::Help => {
                            let from = self.members.get_mut(&username).unwrap().clone();
                            let message = message::System::new(
                                from.user.clone(),
                                format!(
                                    "Available commands: {}{}",
                                    utils::NEWLINE,
                                    Command::to_string()
                                ),
                            );
                            self.send_message(message.into()).await;
                        }
                        Command::Quiet => {
                            let app = self.members.get_mut(&username).unwrap();
                            app.user.switch_quiet_mode();

                            let message = message::System::new(
                                app.user.clone(),
                                match app.user.quiet {
                                    true => "Quiet mode is toggled ON",
                                    false => "Quiet mode is toggled OFF",
                                }
                                .to_string(),
                            );
                            self.send_message(message.into()).await;
                        }
                    }

                    let member = self.members.get_mut(&username).unwrap();
                    member.state.input.clear();
                }
                KeyCode::Backspace => {
                    let member = self.members.get_mut(&username).unwrap();
                    member.state.input.pop();
                }
                KeyCode::CtrlW => {
                    let member = self.members.get_mut(&username).unwrap();
                    member.state.input.remove_last_word();
                }
                KeyCode::Char(_) | KeyCode::Space => {
                    let member = self.members.get_mut(&username).unwrap();
                    member.state.input.extend(data);
                }
                _ => {}
            }
        }
    }

    async fn is_member(&self, username: &UserName) -> bool {
        self.members.contains_key(username)
    }
}