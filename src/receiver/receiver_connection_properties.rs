use std::collections::BTreeSet;
use crate::connection_properties::ConnectionProperties;

pub struct ReceiverConnectionProperties {
    pub static_properties: ConnectionProperties,
    pub window_position: u16,
    pub window_buffer: Vec<u8>,
    pub parts_received: BTreeSet<u16>,
}

impl ReceiverConnectionProperties {
    pub fn new(conn_props: ConnectionProperties) -> Self {
        let buffer_length = (conn_props.window_size * (conn_props.packet_size - conn_props.checksum_size)) as usize;
        Self {
            static_properties: conn_props,
            window_position: 0,
            window_buffer: vec![0; buffer_length],
            parts_received: BTreeSet::new()
        }
    }
}