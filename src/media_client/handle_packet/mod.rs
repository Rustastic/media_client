use colored::Colorize;
use log::error;
use messages::client_commands::MediaClientEvent::{
    DestinationIsDrone, ErrorPacketCache, UnreachableNode,
};
use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{Ack, FloodRequest, FloodResponse, Nack, NackType, NodeType, Packet},
};

use super::MediaClient;

#[cfg(test)]
mod test;

impl MediaClient {
    pub fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type {
            wg_2024::packet::PacketType::MsgFragment(ref fragment) => {
                if self.check_packet(&packet, Some(fragment.fragment_index)) {
                    self.send_ack(fragment.fragment_index, &packet);
                    if let Some(message) = self.message_factory.received_fragment(
                        fragment.clone(),
                        packet.session_id,
                        packet.routing_header.hops[0],
                    ) {
                        self.handle_message(message);
                    }
                } else {
                    let nack = Packet::new_nack(packet.routing_header.get_reversed(), packet.session_id, Nack { fragment_index: fragment.fragment_index, nack_type: NackType::UnexpectedRecipient(self.id) });
                    self.send_packet(nack, None);
                }
            },
            wg_2024::packet::PacketType::Ack(ack) => {
                // self.message_factory.received_ack(ack, packet.session_id);
                self.packet_cache
                    .take_packet((packet.session_id, ack.fragment_index));
            }
            wg_2024::packet::PacketType::Nack(nack) => self.handle_nack(nack, packet.session_id),
            wg_2024::packet::PacketType::FloodRequest(request) => {
                let res = self.get_flood_response(request, packet.session_id);
                self.send_packet(res, None);
            }
            wg_2024::packet::PacketType::FloodResponse(response) => {
                self.router.handle_flood_response(&response);
            }
        }

        todo!()
    }
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
        self.send_packet(packet, None);
    }
    fn check_packet(&self, packet: &Packet, fragment_index: Option<u64>) -> bool {
        let hop_index = packet.routing_header.hop_index;
        if self.id != packet.routing_header.hops[hop_index] {
            let nack_packet = Packet {
                routing_header: packet.routing_header.get_reversed(),
                pack_type: wg_2024::packet::PacketType::Nack(Nack {
                    fragment_index: fragment_index.unwrap_or(0),
                    nack_type: NackType::UnexpectedRecipient(self.id),
                }),
                ..packet.clone()
            };
            self.send_packet(nack_packet, None);
            return false;
        }
        true
    }
    fn get_flood_response(&self, flood_request: FloodRequest, session_id: u64) -> Packet {
        let mut path_trace = flood_request.path_trace;
        path_trace.push((self.id, NodeType::Client));
        let mut hops = path_trace.iter().map(|(id, _)| *id).collect::<Vec<u8>>();
        hops.reverse();
        hops.push(flood_request.initiator_id);
        let flood_response = FloodResponse {
            flood_id: flood_request.flood_id,
            path_trace,
        };
        Packet {
            routing_header: SourceRoutingHeader::with_first_hop(hops),
            session_id,
            pack_type: wg_2024::packet::PacketType::FloodResponse(flood_response),
        }
    }
    fn send_ack(&self, fragment_index: u64, packet: &Packet) {
        let ack = Packet {
            routing_header: packet.routing_header.get_reversed(),
            session_id: packet.session_id,
            pack_type: wg_2024::packet::PacketType::Ack(Ack { fragment_index }),
        };
        self.send_packet(ack, None);
    }
}
