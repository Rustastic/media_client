use std::{collections::HashMap, thread, time::Duration};

use assembler::HighLevelMessageFactory;
use file_assembler::FileAssembler;
use messages::client_commands::{MediaClientCommand, MediaClientEvent};
use packet_cache::PacketCache;
use source_routing::Router;

use colored::Colorize;
use crossbeam_channel::{select_biased, Receiver, Sender};
use log::info;
use wg_2024::{
    network::NodeId,
    packet::{NodeType, Packet},
};

mod handle_command;
mod handle_message;
mod handle_packet;
mod send_to;

mod packet_cache;
mod file_assembler;

pub struct MediaClient {
    id: NodeId,

    router: Router,
    message_factory: HighLevelMessageFactory,

    packet_cache: PacketCache,
    file_assembler: FileAssembler,

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
            file_assembler: FileAssembler::new(),
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
            self.send_packet(req, Some(sender));
        }
        thread::sleep(Duration::from_millis(10));
    }
    // fn get_discovered_server(&self, id: NodeId) -> Option<&DiscoveredServer> {
    //     let index = self.discovered_servers.iter().position(|s| s.id == id)?;
    //     self.discovered_servers.get(index)
    // }
    // fn get_discovered_server_mut(&mut self, id: NodeId) -> Option<&mut DiscoveredServer> {
    //     let index = self.discovered_servers.iter().position(|s| s.id == id)?;
    //     self.discovered_servers.get_mut(index)
    // }
}
