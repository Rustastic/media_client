use std::{collections::HashMap, thread, time::Duration};

use assembler::HighLevelMessageFactory;
use messages::client_commands::{MediaClientCommand, MediaClientEvent};
use packet_cache::PacketCache;
use source_routing::Router;

use colored::Colorize;
use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{error, info};
use wg_2024::{
    network::NodeId,
    packet::{NodeType, Packet},
};

mod handle_command;
mod handle_message;
mod handle_packet;

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
        info!("{} [MediaClient {}] sending packet", "✓".green(), self.id);
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
    /// To be used only with `Ack`, `Nack` and `FloodResponse`
    fn send_or_shortcut(&self, msg: Packet) {
        fn get_sender(this: &MediaClient, packet: &Packet) -> Option<Sender<Packet>> {
            Some(
                this.packet_send
                    .get(&packet.routing_header.next_hop()?)?
                    .clone(),
            )
        }
        match get_sender(self, &msg) {
            Some(sender) => self.send_to_sender(msg, &sender),
            None => self.send_controller(MediaClientEvent::ControllerShortcut(msg)),
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
