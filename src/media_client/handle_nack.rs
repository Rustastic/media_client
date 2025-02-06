use colored::Colorize;
use log::error;
use wg_2024::packet::{Nack, Packet};

use super::MediaClient;

impl MediaClient {
    pub fn handle_nack(&mut self, nack: Nack, session_id: u64) {
        match nack.nack_type {
            wg_2024::packet::NackType::ErrorInRouting(crashed_id) => {
                let _ = self.router.drone_crashed(crashed_id);
                if let Some((old_packet,freq)) = self
                    .packet_cache
                    .get_value((session_id, nack.fragment_index))
                {
                    if freq > 5 {
                        // TODO: clear_network and reflood
                    }
                    let Some(destination) = old_packet.routing_header.destination() else {
                        return;
                    };
                    let Ok(new_header) = self.router.get_source_routing_header(destination) else {
                        self.send_controller(
                            messages::client_commands::MediaClientEvent::UnreachableNode(
                                destination,
                            ),
                        );
                        return;
                    };
                    let new_packet = Packet {
                        routing_header: new_header,
                        session_id: old_packet.session_id,
                        pack_type: old_packet.pack_type,
                    };
                    self.send_packet(new_packet);
                }
            }
            wg_2024::packet::NackType::DestinationIsDrone => error!(
                "{} [ ChatClient {} ]: Destination is drone",
                "✗".red(),
                self.id
            ),
            wg_2024::packet::NackType::Dropped => {
                error!("{} [ChatClient. {}]: Nack dropped", "✗".red(), self.id);
            }
            wg_2024::packet::NackType::UnexpectedRecipient(_) => todo!(),
        }
    }
}
