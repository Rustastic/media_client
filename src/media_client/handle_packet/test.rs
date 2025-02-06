use std::collections::HashMap;

use crossbeam_channel::unbounded;

use super::*;


#[test]
fn test_get_flood_response() {
    let mock_client = MediaClient::new(128, unbounded().0, unbounded().1, unbounded().1, HashMap::new()) ;

    let flood_request = FloodRequest {
        flood_id: 123,
        initiator_id: 129,
        path_trace: vec![
            (1, NodeType::Drone),
            (2, NodeType::Drone),
            (3, NodeType::Drone),
            (4, NodeType::Drone),
            (5, NodeType::Drone)
        ],
    };

    let flood_response = mock_client.get_flood_response(flood_request, 256);
    let expect = Packet { 
        routing_header: SourceRoutingHeader { 
            hop_index: 1, 
            hops: vec![
                128, 5, 4, 3, 2, 1, 129
            ] 
        },
        session_id: 256, 
        pack_type: wg_2024::packet::PacketType::FloodResponse(FloodResponse { 
            flood_id: 123, 
            path_trace: vec![
                (1, NodeType::Drone),
                (2, NodeType::Drone),
                (3, NodeType::Drone),
                (4, NodeType::Drone),
                (5, NodeType::Drone),
                (128, NodeType::Client)
            ] 
        }) 
    } ;

    assert_eq!(
        flood_response, 
        expect
    ) ;

}