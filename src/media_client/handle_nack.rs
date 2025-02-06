use colored::Colorize;
use log::error;
use messages::client_commands::MediaClientEvent::{
    DestinationIsDrone, ErrorPacketCache, UnreachableNode,
};
use wg_2024::{
    network::NodeId,
    packet::{Nack, Packet},
};

use super::MediaClient;

impl MediaClient {
    #[allow(clippy::needless_pass_by_value)] //want to consume the nack
    pub fn handle_nack(&mut self, nack: Nack, session_id: u64) {
        match nack.nack_type {
            wg_2024::packet::NackType::ErrorInRouting(crashed_id) => {
                error!(
                    "{} [MediaClient {}]: error_in_routing({crashed_id})",
                    "✗".red(),
                    self.id,
                );
                let _ = self.router.drone_crashed(crashed_id);
                self.resend_for_nack(session_id, nack.fragment_index, crashed_id);
            }
            wg_2024::packet::NackType::DestinationIsDrone => {
                error!(
                    "{} [MediaClient {}]: Destination is drone",
                    "✗".red(),
                    self.id
                );
                self.send_controller(DestinationIsDrone);
            }
            wg_2024::packet::NackType::Dropped => {
                error!("{} [MediaClient. {}]: Nack dropped", "✗".red(), self.id);
                self.resend_for_nack(session_id, nack.fragment_index, self.id);
            }
            wg_2024::packet::NackType::UnexpectedRecipient(id) => {
                error!(
                    "{} [MediaClient {}] unexpectedRecipient from node: {id}",
                    "✗".red(),
                    self.id
                );
                self.resend_for_nack(session_id, nack.fragment_index, id);
            }
        }
    }
    fn resend_for_nack(&mut self, session_id: u64, fragment_index: u64, nack_src: NodeId) {
        let Some((mut packet, freq)) = self.packet_cache.get_value((session_id, fragment_index))
        else {
            self.send_controller(ErrorPacketCache(session_id, fragment_index));
            return;
        };
        if freq > 10 {
            // consider the drone crashed and reget a header
            let _ = self.router.drone_crashed(nack_src).inspect_err(|_| {
                self.reinit_network();
            });
            let Some(destination) = packet.routing_header.destination() else {
                return;
            };
            let Ok(new_header) = self.router.get_source_routing_header(destination) else {
                self.send_controller(UnreachableNode(destination));
                return;
            };
            self.flood_network();

            let new_packet = Packet {
                routing_header: new_header,
                ..packet
            };
            packet = new_packet;
        } else if freq > 5 {
            // reflood network and reget a header
            self.flood_network();
            let Some(destination) = packet.routing_header.destination() else {
                return;
            };
            let Ok(new_header) = self.router.get_source_routing_header(destination) else {
                self.send_controller(UnreachableNode(destination));
                return;
            };
            let new_packet = Packet {
                routing_header: new_header,
                ..packet
            };
            packet = new_packet;
        }
        self.send_packet(packet);
    }
}
