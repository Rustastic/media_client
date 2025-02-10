use colored::Colorize;
use log::{error, warn};
use messages::high_level_messages::MessageContent::FromClient;
use messages::{
    client_commands::{MediaClientCommand, MediaClientEvent},
    high_level_messages::ClientMessage,
};
use wg_2024::network::NodeId;

use super::MediaClient;

impl MediaClient {
    pub fn handle_command(&mut self, command: MediaClientCommand) {
        match command {
            MediaClientCommand::InitFlooding => self.flood_network(),
            MediaClientCommand::RemoveSender(id) => {
                let _ = self
                    .packet_send
                    .remove(&id)
                    .inspect(|_| {
                        self.send_controller(MediaClientEvent::RemovedSender(id));
                    })
                    .ok_or(())
                    .inspect_err(|()| {
                        warn!(
                            "{} [ MediaClient {} ] is already disconnected from [ Drone {id} ]",
                            "!!!".yellow(),
                            self.id
                        );
                    });
            }
            MediaClientCommand::AddSender(id, sender) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.packet_send.entry(id) {
                    e.insert(sender);
                    self.send_controller(MediaClientEvent::AddedSender(id));
                } else {
                    warn!(
                        "{} [ MediaClient {} ] is already connected to [ Drone {id} ]",
                        "!!!".yellow(),
                        self.id
                    );
                }
            }
            MediaClientCommand::GetServerList => {
                let server_list = self
                    .router
                    .get_server_list()
                    .into_iter()
                    .collect::<Vec<NodeId>>();
                self.send_controller(MediaClientEvent::ServerList(server_list.clone()));
                for server in server_list {
                    let Ok(header) = self.router.get_source_routing_header(server) else {
                        continue;
                    };
                    let message = self.message_factory.get_message_from_message_content(FromClient(ClientMessage::GetServerType), &header, server);
                    for fragment in message {
                        self.send_packet(fragment, None);
                    }
                }
            }
            MediaClientCommand::AskServerType(id)
            | MediaClientCommand::AskFilesList(id)
            | MediaClientCommand::AskForFile(id, _) => self.handle_ask(id, command),
        }
    }
    fn handle_ask(&mut self, destination: NodeId, command: MediaClientCommand) {
        let Ok(header) = self.router.get_source_routing_header(destination) else {
            self.send_controller(MediaClientEvent::UnreachableNode(destination));
            error!(
                "{} [ MediaClient {} ]: Cannot send message, destination {destination} is unreachable",
                "✗".red(),
                self.id,
            );
            return;
        };
        let client_message = match command {
            MediaClientCommand::AskServerType(_) => ClientMessage::GetServerType,
            MediaClientCommand::AskFilesList(_) => ClientMessage::GetFilesList,
            MediaClientCommand::AskForFile(_, file_id) => ClientMessage::GetFile(file_id),
            _ => return,
        };
        for fragment_packet in self.message_factory.get_message_from_message_content(
            FromClient(client_message),
            &header,
            destination,
        ) {
            self.packet_cache.insert_packet(&fragment_packet);
            self.send_packet(fragment_packet, None);
        }
    }
}
