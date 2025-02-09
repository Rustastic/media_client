use messages::high_level_messages::{
    ClientMessage::GetMedia, Message, MessageContent::{FromClient, FromServer}, ServerMessage::{File, FilesList, Media, ServerType}
};

use crate::media_client::file_assembler::AddedFileReturn::{CompleteFile, RefToMedia};

use super::MediaClient;

impl MediaClient {
    pub fn handle_message(&mut self, message: Message) {
        let FromServer(content) = message.content else {
            return;
        };
        match content {
            ServerType(server_type) => {
                self.send_controller(messages::client_commands::MediaClientEvent::ReceveidServerType(message.source_id, server_type));
            }
            FilesList(files_ids) => {
               self.send_controller(messages::client_commands::MediaClientEvent::ReceveidFileList(message.source_id, files_ids));
            }
            File {
                file_id,
                size,
                content,
            } => {
                match self.file_assembler.add_textfile(message.source_id, &file_id, content, size) {
                    CompleteFile { source_id: _, file_id: _, content: _, media_content: _ } => todo!("send to sim controller"),
                    RefToMedia(items) => {
                        for (node_id, file_id) in items {
                            let Ok(header) = self.router.get_source_routing_header(node_id) else {
                                continue;
                            };
                            let message = self.message_factory.get_message_from_message_content(FromClient(GetMedia(file_id)), &header, node_id);
                            for fragment in message {
                                self.packet_cache.insert_packet(&fragment);
                                self.send_packet(fragment, None);
                            }
                        }
                    },
                }
            }
            Media(media_id, content) => {
                let complete_file = self.file_assembler.add_media_file(message.source_id, &media_id, content) ;
                if let Some(_complete_file) = complete_file {
                    todo!("send to sim controller");
                }
            },
            _ => (),
        }
    }
}
