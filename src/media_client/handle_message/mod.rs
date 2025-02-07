use messages::high_level_messages::{Message, MessageContent::FromServer, ServerMessage::{File, FilesList, Media, ServerType}};

use super::MediaClient;

impl MediaClient {
    pub fn handle_message(&self, message: Message) {
        let FromServer(content) = message.content else {
            return;
        };
        match content {
            ServerType(server_type) => todo!(),
            FilesList(items) => todo!(),
            File { size, content } => todo!(),
            Media(items) => todo!(),
            _ => return,
        }




        todo!()
    }
}
