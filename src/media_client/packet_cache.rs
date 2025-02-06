use std::{cell::RefCell, collections::HashMap};

use wg_2024::packet::Packet;

/// (`session_id`, `fragment_id`)
pub type Key = (u64, u64);
/// (`packet`, `number_of_request`)
pub type Value = (Packet, u64);

pub struct PacketCache {
    cache: RefCell<HashMap<Key, Value>>,
}

impl PacketCache {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }
    pub fn insert_packet(&self, packet: &Packet) {
        let session_id = packet.session_id;
        if let wg_2024::packet::PacketType::MsgFragment(frg) = &packet.pack_type {
            let fragment_id = frg.fragment_index;
            self.cache
                .borrow_mut()
                .insert((session_id, fragment_id), (packet.clone(), 0));
        }
    }
    pub fn insert_value(&self, value: &Value) {
        let session_id = value.0.session_id;
        if let wg_2024::packet::PacketType::MsgFragment(frg) = &value.0.pack_type {
            let fragment_id = frg.fragment_index;
            self.cache
                .borrow_mut()
                .insert((session_id, fragment_id), value.clone());
        }
    }
    pub fn get_packet(&self, key: Key) -> Option<Packet> {
        let mut cache = self.cache.try_borrow_mut().ok()?;
        let value = cache.get_mut(&key)?;
        value.1 += 1;
        Some(value.0.clone())
    }
    pub fn get_value(&self, key: Key) -> Option<Value> {
        let mut cache = self.cache.try_borrow_mut().ok()?;
        let value = cache.get_mut(&key)?;
        value.1 += 1;
        Some(value.clone())
    }
    pub fn take_packet(&self, key: Key) -> Option<Packet> {
        self.cache
            .try_borrow_mut()
            .ok()?
            .remove(&key)
            .map(|(p, _)| p)
    }
}
