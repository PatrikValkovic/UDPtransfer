use std::net::SocketAddr;

pub struct ConnectionProperties {
    pub id: u32,
    pub checksum_size: u16,
    pub window_size: u16,
    pub packet_size: u16,
    pub socket_addr: SocketAddr
}

impl ConnectionProperties {
    pub fn new(id: u32, checksum_size: u16, window_size: u16, packet_size: u16, socket_addr: SocketAddr) -> Self {
        ConnectionProperties {
            id,
            checksum_size,
            window_size,
            packet_size,
            socket_addr
        }
    }
}