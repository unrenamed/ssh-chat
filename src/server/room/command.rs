use super::user::{Theme, TimestampMode};
use crate::utils;

use fmt::Write;
use std::{fmt, str::FromStr};
use strum::{EnumCount, EnumIter, EnumProperty, IntoEnumIterator};

#[derive(Debug, Clone, PartialEq, EnumProperty, EnumIter, EnumCount)]
pub enum Command {
    #[strum(props(Cmd = "/exit", Help = "Exit the chat application"))]
    Exit,

    #[strum(props(
        Cmd = "/away",
        Args = "<reason>",
        Help = "Let the room know you can't make it and why"
    ))]
    Away(String),

    #[strum(props(Cmd = "/back", Help = "Clear away status"))]
    Back,

    #[strum(props(Cmd = "/name", Args = "<name>", Help = "Rename yourself"))]
    Name(String),

    #[strum(props(
        Cmd = "/msg",
        Args = "<user> <message>",
        Help = "Send a private message to a user"
    ))]
    Msg(String, String),

    #[strum(props(
        Cmd = "/reply",
        Args = "<message>",
        Help = "Reply to the previous private message"
    ))]
    Reply(String),

    #[strum(props(Cmd = "/ignore", Args = "[user]", Help = "Hide messages from a user"))]
    Ignore(Option<String>),

    #[strum(props(
        Cmd = "/unignore",
        Args = "<user>",
        Help = "Stop hidding messages from a user"
    ))]
    Unignore(String),

    #[strum(props(
        Cmd = "/focus",
        Args = "[user]",
        Help = "Only show messages from focused users. $ to reset"
    ))]
    Focus(Option<String>),

    #[strum(props(Cmd = "/users", Help = "List users who are connected"))]
    Users,

    #[strum(props(Cmd = "/whois", Args = "<user>", Help = "Information about a user"))]
    Whois(String),

    #[strum(props(
        Cmd = "/timestamp",
        Args = "<time|datetime>",
        Help = "Prefix messages with a UTC timestamp"
    ))]
    Timestamp(TimestampMode),

    #[strum(props(Cmd = "/theme", Args = "<theme>", Help = "Set your color theme"))]
    Theme(Theme),

    #[strum(props(Cmd = "/themes", Help = "List supported color themes"))]
    Themes,

    #[strum(props(Cmd = "/quiet", Help = "Silence room announcements"))]
    Quiet,

    /// Operator commands

    #[strum(props(
        Cmd = "/mute",
        Args = "<user>",
        Help = "Toggle muting user, preventing messages from broadcasting",
        Op = "true"
    ))]
    Mute(String),

    #[strum(props(
        Cmd = "/kick",
        Args = "<user>",
        Help = "Kick user from the server",
        Op = "true"
    ))]
    Kick(String),

    #[strum(props(
        Cmd = "/ban",
        Args = "<query>",
        Help = "Ban user from the server",
        Op = "true"
    ))]
    Ban(String),

    #[strum(props(Cmd = "/banned", Help = "List the current ban conditions", Op = "true"))]
    Banned,

    #[strum(props(
        Cmd = "/motd",
        Args = "[message]",
        Help = "Set a new message of the day, or print the motd if no message",
        Op = "true"
    ))]
    Motd(Option<String>),

    /// Secret commands (just hidden or easter eggs)

    #[strum(props(Cmd = "/me", Args = "[action]"))]
    Me(Option<String>),

    #[strum(props(Cmd = "/slap", Args = "[user]"))]
    Slap(Option<String>),

    #[strum(props(Cmd = "/shrug",))]
    Shrug,

    #[strum(props(Cmd = "/help"))]
    Help,

    #[strum(props(Cmd = "/version"))]
    Version,

    #[strum(props(Cmd = "/uptime"))]
    Uptime,
}

#[derive(Debug, PartialEq)]
pub enum CommandParseError {
    NotRecognizedAsCommand,
    UnknownCommand,
    ArgumentExpected(String),
    Custom(String),
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandParseError::NotRecognizedAsCommand => write!(f, "given input is not a command"),
            CommandParseError::UnknownCommand => write!(f, "unknown command"),
            CommandParseError::ArgumentExpected(arg) => write!(f, "{} is expected", arg),
            CommandParseError::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (cmd, args) = if let Some((cmd, args)) = s.split_once(' ') {
            (cmd, args.trim_start())
        } else {
            (s, "")
        };

        if !cmd.starts_with("/") {
            return Err(Self::Err::NotRecognizedAsCommand);
        }

        match cmd.as_bytes() {
            b"/exit" => Ok(Command::Exit),
            b"/away" => match args.is_empty() {
                true => Err(Self::Err::ArgumentExpected(format!("away reason"))),
                false => Ok(Command::Away(args.to_string())),
            },
            b"/back" => Ok(Command::Back),
            b"/name" => match args.splitn(2, ' ').nth(0) {
                Some(new_name) => Ok(Command::Name(new_name.to_string())),
                None => Err(Self::Err::ArgumentExpected(format!("new name"))),
            },
            b"/msg" => {
                let mut iter = args.splitn(2, ' ');
                let user = iter.next();
                if user.is_none() || user.unwrap().is_empty() {
                    return Err(Self::Err::ArgumentExpected(format!("user name")));
                }
                let body = iter.next();
                if body.is_none() || body.unwrap().is_empty() {
                    return Err(Self::Err::ArgumentExpected(format!("message body")));
                };
                Ok(Command::Msg(
                    user.unwrap().to_string(),
                    body.unwrap().trim_start().to_string(),
                ))
            }
            b"/reply" => {
                if args.is_empty() {
                    return Err(Self::Err::ArgumentExpected(format!("message body")));
                };
                Ok(Command::Reply(args.to_string()))
            }
            b"/users" => Ok(Command::Users),
            b"/whois" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Whois(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/slap" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => Ok(Command::Slap(None)),
                Some(user) => Ok(Command::Slap(Some(user.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/shrug" => Ok(Command::Shrug),
            b"/quiet" => Ok(Command::Quiet),
            b"/me" => match args.is_empty() {
                true => Ok(Command::Me(None)),
                false => Ok(Command::Me(Some(args.to_string()))),
            },
            b"/timestamp" => match args.splitn(2, ' ').nth(0) {
                Some(mode) if mode.is_empty() => Err(Self::Err::Custom(
                    "timestamp value must be one of: time, datetime, off".to_string(),
                )),
                Some(mode) => match mode {
                    "time" | "datetime" | "off" => {
                        Ok(Command::Timestamp(TimestampMode::from_str(mode).unwrap()))
                    }
                    _ => Err(Self::Err::Custom(
                        "timestamp value must be one of: time, datetime, off".to_string(),
                    )),
                },
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/theme" => match args.splitn(2, ' ').nth(0) {
                Some(theme) if theme.is_empty() => Err(Self::Err::Custom(format!(
                    "theme value must be one of: {}",
                    Theme::all().join(", ")
                ))),
                Some(theme) => {
                    let supported_themes = Theme::all();
                    if supported_themes.contains(&theme.to_string()) {
                        Ok(Command::Theme(Theme::from_str(theme).unwrap()))
                    } else {
                        Err(Self::Err::Custom(format!(
                            "theme value must be one of: {}",
                            Theme::all().join(", ")
                        )))
                    }
                }
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/themes" => Ok(Command::Themes),
            b"/ignore" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => Ok(Command::Ignore(None)),
                Some(user) => Ok(Command::Ignore(Some(user.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/unignore" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Unignore(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/focus" => match args.splitn(2, ' ').nth(0) {
                Some(users) if users.is_empty() => Ok(Command::Focus(None)),
                Some(users) => Ok(Command::Focus(Some(users.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/help" => Ok(Command::Help),
            b"/version" => Ok(Command::Version),
            b"/uptime" => Ok(Command::Uptime),
            b"/mute" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Mute(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/motd" => Ok(match args.is_empty() {
                true => Command::Motd(None),
                false => Command::Motd(Some(args.to_string())),
            }),
            b"/kick" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Kick(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/ban" => {
                if args.is_empty() {
                    return Err(Self::Err::ArgumentExpected(format!("ban query")));
                };
                Ok(Command::Ban(args.to_string()))
            }
            b"/banned" => Ok(Command::Banned),
            _ => Err(Self::Err::UnknownCommand),
        }
    }
}

impl Command {
    pub fn cmd(&self) -> &str {
        self.get_str("Cmd").unwrap_or_default()
    }

    pub fn args(&self) -> &str {
        self.get_str("Args").unwrap_or_default()
    }

    pub fn help(&self) -> &str {
        self.get_str("Help").unwrap_or_default()
    }

    pub fn is_op(&self) -> bool {
        self.get_str("Op").unwrap_or_default() == "true"
    }

    pub fn is_visible(&self) -> bool {
        !self.help().is_empty()
    }
}

pub struct CommandCollection {
    commands: Vec<Command>,
}

impl CommandCollection {
    pub fn new() -> Self {
        let mut commands: Vec<Command> = Command::iter().collect();
        commands.sort_by(|a, b| a.cmd().len().cmp(&b.cmd().len()));
        Self { commands }
    }

    pub fn to_string(&self, show_op: bool) -> String {
        let mut result = format!("Available commands: {}", utils::NEWLINE);

        let noop_count = self.noop_visible_iter().count();
        let noop_commands = self.noop_visible_iter();
        result.push_str(&self.format(noop_commands, noop_count));

        if show_op {
            result.push_str(&format!(
                "{}{}Operator commands: {}",
                utils::NEWLINE,
                utils::NEWLINE,
                utils::NEWLINE
            ));
            let op_count = self.op_visible_iter().count();
            let op_commands = self.op_visible_iter();
            result.push_str(&self.format(op_commands, op_count));
        }

        result
    }

    pub fn from_prefix(&self, prefix: &str) -> Option<&Command> {
        for cmd in &self.commands {
            if cmd.cmd().starts_with(prefix) {
                return Some(cmd);
            }
        }
        None
    }

    fn format<'a, I>(&self, commands: I, count: usize) -> String
    where
        I: Iterator<Item = &'a Command> + 'a,
    {
        let mut result = String::new();
        for (idx, cmd) in commands.enumerate() {
            write!(
                result,
                "{:<10} {:<20} {}{}",
                cmd.cmd(),
                cmd.args(),
                cmd.help(),
                if idx == count - 1 { "" } else { utils::NEWLINE }
            )
            .unwrap();
        }

        result
    }

    fn all_visible_iter(&self) -> impl Iterator<Item = &Command> {
        self.commands.iter().filter(|cmd| cmd.is_visible())
    }

    fn noop_visible_iter(&self) -> impl Iterator<Item = &Command> {
        self.all_visible_iter().filter(|c| !c.is_op())
    }

    fn op_visible_iter(&self) -> impl Iterator<Item = &Command> {
        self.all_visible_iter().filter(|c| c.is_op())
    }
}
