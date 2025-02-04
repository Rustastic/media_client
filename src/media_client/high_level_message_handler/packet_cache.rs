use log::error;
use std::{cell::RefCell, collections::HashMap};
use wg_2024::packet::{Packet, PacketType};

pub struct PacketCache {
    // (session_id, fragment_id) , (packet, number_of_request)
    cache: RefCell<HashMap<(u64, u64), (Packet, u64)>>,
}

impl PacketCache {
    //constructor
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }
}

impl PacketCache {
    //methods
    pub fn insert(&self, packet: &Packet) {
        let session_id = packet.session_id;
        if let PacketType::MsgFragment(fragment) = &packet.pack_type {
            let fragment_index = fragment.fragment_index;
            self.cache
                .borrow_mut()
                .insert((session_id, fragment_index), (packet.clone(), 0));
        } else {
            error!("Trying to insert a packet that doesn't contain a fragment");
        }
    }
    pub fn remove(&self, session_id: u64, fragment_index: u64) -> bool {
        self.cache
            .borrow_mut()
            .remove(&(session_id, fragment_index))
            .is_some()
    }
}

impl PacketCache {
    //getter/setter
    pub fn get(&self, session_id: u64, fragment_index: u64) -> Option<Packet> {
        let mut cache = self.cache.try_borrow_mut().ok()?;
        let value = cache.get_mut(&(session_id, fragment_index))?;
        value.1 += 1;
        Some(value.0.clone())
    }
    pub fn take(&self, session_id: u64, fragment_index: u64) -> Option<Packet> {
        self.cache
            .borrow_mut()
            .remove(&(session_id, fragment_index))
            .map(|v| v.0)
    }
}
