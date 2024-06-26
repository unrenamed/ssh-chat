use async_trait::async_trait;
use chrono::Utc;

use crate::server::room::Command;
use crate::server::room::{message, CommandParseError};
use crate::server::terminal::Terminal;
use crate::server::ServerRoom;

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

#[derive(Default)]
pub struct CommandParser {
    next: Option<Box<dyn WorkflowHandler>>,
}

impl CommandParser {
    pub fn new(next: impl WorkflowHandler + 'static) -> Self {
        Self {
            next: into_next(next),
        }
    }
}

#[async_trait]
impl WorkflowHandler for CommandParser {
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
        room: &mut ServerRoom,
    ) {
        let user = context.user.clone();

        if context.command_str.is_none() {
            return;
        }

        let command_str = context.command_str.as_ref().unwrap();
        let input_str = terminal.input.to_string();

        match command_str.parse::<Command>() {
            Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                terminal.clear_input().unwrap();
                room.find_member_mut(&user.username)
                    .update_last_sent_time(Utc::now());
                let message = message::Public::new(user, input_str);
                room.send_message(message.into()).await;
            }
            Err(err) => {
                terminal.input.push_to_history();
                terminal.clear_input().unwrap();
                let message = message::Command::new(user.clone(), input_str);
                room.send_message(message.into()).await;
                let message = message::Error::new(user, format!("{}", err));
                room.send_message(message.into()).await;
            }
            Ok(command) => {
                terminal.input.push_to_history();
                terminal.clear_input().unwrap();
                let message = message::Command::new(user.clone(), input_str);
                room.send_message(message.into()).await;
                context.command = Some(command);
            }
        }
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
