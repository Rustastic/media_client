use log::{error, info, log};
use messages::high_level_messages::{
    ClientMessage::{self, GetMedia},
    Message,
    MessageContent::{FromClient, FromServer},
    ServerMessage::{File, FilesList, Media, ServerType},
    ServerType::{Chat, Text},
};

use super::MediaClient;

impl MediaClient {
    pub fn handle_message(&mut self, message: Message) {
        let FromServer(content) = message.content else {
            return;
        };
        match content {
            ServerType(server_type) => {
                match server_type {
                    messages::high_level_messages::ServerType::Media => {
                        self.media_server.insert(message.source_id);
                    }
                    Text | Chat => (),
                }
                self.send_controller(
                    messages::client_commands::MediaClientEvent::ReceveidServerType(
                        message.source_id,
                        server_type,
                    ),
                );
            }
            FilesList(files_ids) => {
                self.send_controller(
                    messages::client_commands::MediaClientEvent::ReceveidFileList(
                        message.source_id,
                        self.id,
                        files_ids,
                    ),
                );
                self.ask_media_server();
            }
            File {
                file_id,
                size,
                content,
            } => {
                info!(
                    "[MediaClient {}] received file: {file_id}", 
                    self.id
                ) ;
                match self
                    .file_assembler
                    .add_textfile(message.source_id, &file_id, content, size)
                {
                    None => info!("[MediaClient {}] file with no ref", self.id),
                    Some(media_ref) => {
                        info!("[MediaClient {}] media_ref: {media_ref:?}", self.id) ;
                        let mut possible_dest = self.media_server.iter().cycle();
                        for (_, file_id) in media_ref {
                            info!(
                                "[MediaClient {}], fetching ref: {file_id}", self.id
                            );
                            let destination = possible_dest
                                .next()
                                .copied()
                                .unwrap_or(*self.media_server.get(&0).unwrap());
                            info!(
                                "[MediaClient: {}] fetching ref: {destination}, {file_id}",
                                self.id
                            );
                            let Ok(header) = self.router.get_source_routing_header(destination)
                            else {
                                error!(
                                    "[MediaClient {}] destination: {destination} unrecheable",
                                    self.id
                                );
                                continue;
                            };
                            let message = self.message_factory.get_message_from_message_content(
                                FromClient(GetMedia(file_id)),
                                &header,
                                destination,
                            );
                            for fragment in message {
                                self.packet_cache.insert_packet(&fragment);
                                self.send_packet(fragment, None);
                            }
                        }
                    }
                }
            }
            Media(media_id, content) => {
                self.file_assembler.add_media_file(&media_id, content);
            }
            _ => (),
        }
    }
    fn ask_media_server(&mut self) {
        for server in self.router.get_server_list() {
            let Ok(header) = self.router.get_source_routing_header(server) else {
                continue;
            };
            let message = self.message_factory.get_message_from_message_content(
                FromClient(ClientMessage::GetServerType),
                &header,
                server,
            );
            for fragment in message {
                self.send_packet(fragment, None);
            }
        }
    }
}
