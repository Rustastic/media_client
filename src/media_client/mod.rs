use std::{collections::HashMap, thread, time::Duration};

use assembler::HighLevelMessageFactory;
use messages::{
    client_commands::{MediaClientCommand, MediaClientEvent},
    high_level_messages::{ClientMessage, MessageContent::FromClient},
};
use packet_cache::PacketCache;
use source_routing::Router;

use colored::Colorize;
use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{error, info, warn};
use wg_2024::{
    network::NodeId,
    packet::{NodeType, Packet},
};

mod handle_nack;
mod packet_cache;

struct MediaClient {
    id: NodeId,

    router: Router,
    message_factory: HighLevelMessageFactory,

    packet_cache: PacketCache,

    controller_send: Sender<MediaClientEvent>,
    controller_recv: Receiver<MediaClientCommand>,

    packet_recv: Receiver<Packet>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
}

impl MediaClient {
    //constructor
    pub fn new(
        id: NodeId,
        controller_send: Sender<MediaClientEvent>,
        controller_recv: Receiver<MediaClientCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        Self {
            id,
            router: Router::new(id, NodeType::Client),
            message_factory: HighLevelMessageFactory::new(id, NodeType::Client),
            packet_cache: PacketCache::new(),
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
        }
    }
}

impl MediaClient {
    //methods
    pub fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command) ;
                    }
                } ,
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet) ;
                    }
                }
            }
        }
    }
    fn send_controller(&self, msg: MediaClientEvent) {
        self.controller_send
            .send(msg)
            .inspect_err(|e| {
                error!(
                    "{} [MediaClient {}] error in sending to sim-controller. Message: [{:?}]",
                    "✗".red(),
                    self.id,
                    e.0
                );
            })
            .ok();
    }
    fn send_packet(&self, msg: Packet) {
        let Some(dest) = msg.routing_header.next_hop() else {
            return;
        };
        self.send_to_neighbour_id(msg, dest);
    }
    fn send_to_sender(&self, msg: Packet, sender: &Sender<Packet>) {
        sender
            .send(msg)
            .inspect_err(|e| {
                self.send_controller(MediaClientEvent::SendError(e.clone()));
                error!(
                    "{} [MediaClient {}] error in sending packet (session: {}, fragment: {})",
                    "✗".red(),
                    self.id,
                    e.0.session_id,
                    e.0.get_fragment_index()
                );
            })
            .ok();
    }
    fn send_to_neighbour_id(&self, msg: Packet, neighbour_id: NodeId) {
        let Some(sender) = self.packet_send.get(&neighbour_id) else {
            self.send_controller(MediaClientEvent::UnreachableNode(neighbour_id));
            error!(
                "{} [ MediaClient {} ]: Cannot send message, destination {neighbour_id} is unreachable",
                "✗".red(),
                self.id,
            );
            return;
        };
        self.send_to_sender(msg, sender);
    }
    fn handle_command(&mut self, command: MediaClientCommand) {
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
            MediaClientCommand::AskServerType(id)
            | MediaClientCommand::AskFilesList(id)
            | MediaClientCommand::AskForMedia(id, _)
            | MediaClientCommand::AskForFile(id, _) => self.handle_ask(id, command),
        }
    }
    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type {
            wg_2024::packet::PacketType::MsgFragment(fragment) => {
                self.message_factory.received_fragment(
                    fragment,
                    packet.session_id,
                    packet.routing_header.hops[0],
                );
            }
            wg_2024::packet::PacketType::Ack(ack) => {
                // self.message_factory.received_ack(ack, packet.session_id);
                self.packet_cache
                    .take_packet((packet.session_id, ack.fragment_index));
            }
            wg_2024::packet::PacketType::Nack(nack) => self.handle_nack(nack, packet.session_id),
            wg_2024::packet::PacketType::FloodRequest(_flood_request) => todo!(),
            wg_2024::packet::PacketType::FloodResponse(_flood_response) => todo!(),
        }

        todo!()
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
        let Some(sender) = self.packet_send.get(&header.next_hop().unwrap_or(self.id)) else {
            self.send_controller(MediaClientEvent::UnreachableNode(
                header.next_hop().unwrap_or(destination),
            ));
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
            MediaClientCommand::AskForMedia(_, media_id) => ClientMessage::GetMedia(media_id),
            _ => return,
        };
        for fragment_packet in self.message_factory.get_message_from_message_content(
            FromClient(client_message),
            &header,
            destination,
        ) {
            self.packet_cache.insert_packet(&fragment_packet);
            self.send_to_sender(fragment_packet, sender);
        }
    }
    fn reinit_network(&mut self) {
        info!(
            "{} [ Mediaclient {} ]: reinitializing the network...",
            "✓".green(),
            self.id
        );
        self.router.clear_routing_table();
        self.flood_network();
    }
    fn flood_network(&self) {
        for sender in self.packet_send.values() {
            let req = self.router.get_flood_request();
            self.send_to_sender(req, sender);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

impl MediaClient {}
