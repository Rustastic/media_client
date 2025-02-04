use std::cell::RefCell;

use assembler::Assembler;
use messages::high_level_messages::{Message, MessageContent};
use packet_cache::PacketCache;
use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{Fragment, NodeType, Packet},
};

mod packet_cache;

pub struct HighLevelMessageHandler {
    id: NodeId,
    node_type: NodeType,
    assembler: Assembler,
    session_id: RefCell<u64>,
    packet_cache: PacketCache,
    // controller_send: Sender<MediaClientEvent>,
    // packet_send: HashMap<NodeId, Sender<Packet>>,
}

impl HighLevelMessageHandler {
    //constructor
    pub fn new(
        id: NodeId,
        node_type: NodeType, /* controller_send: &Sender<MediaClientEvent>, packet_send: &HashMap<NodeId, Sender<Packet>> */
    ) -> Self {
        Self {
            id,
            node_type,
            assembler: Assembler::new(),
            session_id: RefCell::new(0),
            packet_cache: PacketCache::new(),
            // controller_send: controller_send.clone(),
            // packet_send: packet_send.clone(),
        }
    }
}

impl HighLevelMessageHandler {
    //methods
    /*
    pub fn add_neighbour(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
    pub fn remove_neighbour(&mut self, id: NodeId) {
        self.packet_send.remove(&id);
    }
    */
    pub fn get_message_from_message_content(
        &mut self,
        message_content: MessageContent,
        header: &SourceRoutingHeader,
        // sender: &Sender<Packet>,
    ) -> Vec<Packet> {
        let session_id = self.get_session_id();
        let message =
            self.create_message(message_content, session_id, *header.hops.last().unwrap());

        let mut packets = Vec::new();
        if let Ok(fragments) = self.assembler.fragment_message(&message) {
            for fragment in fragments {
                let packet = Packet::new_fragment(header.clone(), session_id, fragment);
                self.packet_cache.insert(&packet);
                // sender.send(packet);
                packets.push(packet);
            }
        }
        packets
    }
    pub fn received_fragment(
        &mut self,
        fragment: Fragment,
        session_id: u64,
        source_id: NodeId,
    ) -> Option<Message> {
        self.assembler
            .process_fragment(fragment, session_id, source_id)
    }
    fn create_message(
        &self,
        message_content: MessageContent,
        session_id: u64,
        destination_id: NodeId,
    ) -> Message {
        match message_content {
            MessageContent::FromClient(client_message) => {
                Message::new_client_message(session_id, self.id, destination_id, client_message)
            }
            MessageContent::FromServer(server_message) => {
                Message::new_server_message(session_id, self.id, destination_id, server_message)
            }
        }
    }
}

impl HighLevelMessageHandler {
    //getter/setter
    fn get_session_id(&self) -> u64 {
        let ret = *self.session_id.borrow();
        *self.session_id.borrow_mut() = ret + 1;
        ret
    }
}
