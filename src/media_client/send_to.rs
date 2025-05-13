use colored::Colorize;
use crossbeam_channel::Sender;
use log::{error, info};
use messages::client_commands::MediaClientEvent;
use wg_2024::{network::NodeId, packet::Packet};

use super::MediaClient;

impl MediaClient {
    pub fn send_controller(&self, msg: MediaClientEvent) {
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
    /// Used to send packet
    ///
    /// # Arguments
    /// -  `sender` : to be included only if  `msg`  is of type  `FloodRequest` otherwise it will be ignored
    pub fn send_packet(&self, msg: Packet, sender: Option<&Sender<Packet>>) {
        match msg.pack_type {
            wg_2024::packet::PacketType::Ack(_)
            | wg_2024::packet::PacketType::Nack(_)
            | wg_2024::packet::PacketType::FloodResponse(_) => self.send_or_shortcut(msg),
            wg_2024::packet::PacketType::FloodRequest(_) => {
                let Some(sender) = sender else { return };
                self.send_to_sender(msg, sender);
            }
            wg_2024::packet::PacketType::MsgFragment(_) => {
                let Some(dest) = msg.routing_header.current_hop() else {
                    error!(
                        "{} [MediaClient {}] error taking next_hop",
                        "✗".red(),
                        self.id
                    );
                    println!(
                        "{} [MediaClient {}] error taking next_hop on msg_frgmt",
                        "✗".red(),
                        self.id
                    );
                    return;
                };
                info!(
                    "{} [MediaClient {}] sending packet to neighbour {dest}",
                    "✓".green(),
                    self.id
                );
                self.send_to_neighbour_id(msg, dest);
            }
        }
    }
    fn send_to_sender(&self, msg: Packet, sender: &Sender<Packet>) {
        info!("{} [MediaClient {}] sending packet", "✓".green(), self.id);
        sender
            .send(msg.clone())
            .inspect(|()| {
                println!(
                    "[Mediaclient {}] sended msg (session: {}, fragment: {})",
                    self.id,
                    msg.session_id,
                    msg.get_fragment_index()
                );
            })
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
        match self.get_sender(&msg) {
            Some(sender) => {
                sender
                    .send(msg)
                    .inspect_err(|e| {
                        self.send_controller(MediaClientEvent::ControllerShortcut(e.0.clone()));
                    })
                    .ok();
            }
            None => self.send_controller(MediaClientEvent::ControllerShortcut(msg)),
        }
    }
    fn get_sender(&self, packet: &Packet) -> Option<Sender<Packet>> {
        Some(
            self.packet_send
                .get(&packet.routing_header.current_hop()?)?
                .clone(),
        )
    }
}
