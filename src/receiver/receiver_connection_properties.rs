use std::collections::BTreeSet;
use std::time::{Instant, Duration};
use crate::connection_properties::ConnectionProperties;

pub struct ReceiverConnectionProperties {
    pub static_properties: ConnectionProperties,
    pub window_position: u16,
    pub window_buffer: Vec<u8>,
    pub parts_received: BTreeSet<u16>,
    pub last_receive_time: Instant,
}

impl ReceiverConnectionProperties {
    pub fn new(conn_props: ConnectionProperties) -> Self {
        let buffer_length = (conn_props.window_size * (conn_props.packet_size - conn_props.checksum_size)) as usize;
        Self {
            static_properties: conn_props,
            window_position: 0,
            window_buffer: vec![0; buffer_length],
            parts_received: BTreeSet::new(),
            last_receive_time: Instant::now()
        }
    }

    pub fn timeouted(&self, timeout: u32) -> bool {
        let threshold_time = Instant::now() - Duration::from_millis(timeout as u64);
        return self.last_receive_time < threshold_time;
    }
}