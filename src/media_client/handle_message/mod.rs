use messages::high_level_messages::{
    Message,
    MessageContent::FromServer,
    ServerMessage::{File, FilesList, Media, ServerType},
};

use super::MediaClient;

impl MediaClient {
    pub fn handle_message(&mut self, message: Message) {
        let FromServer(content) = message.content else {
            return;
        };
        let Some(server) = self.get_discovered_server_mut(message.source_id) else {
            return;
        };
        match content {
            ServerType(server_type) => {
                server.set_server_type(server_type);
                self.send_controller(
                    messages::client_commands::MediaClientEvent::ReceveidServerType(
                        message.source_id,
                        server_type,
                    ),
                );
            }
            FilesList(files_ids) => {
                server.set_files_list(files_ids.clone());
                self.send_controller(
                    messages::client_commands::MediaClientEvent::ReceveidFileList(
                        message.source_id,
                        files_ids,
                    ),
                );
            }
            File {
                file_id,
                size,
                content,
            } => {}
            Media(media_id, items) => todo!(),
            _ => return,
        }

        todo!()
    }
}
