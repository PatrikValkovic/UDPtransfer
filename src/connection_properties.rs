use std::net::SocketAddr;
use crate::loggable::Loggable;
use std::num::Wrapping;

/// Properties that does not change during transmission.
/// The received and sender agree on them beforehand.
pub struct ConnectionProperties {
    /// Connection identifier.
    pub id: u32,
    /// Size of the checksum part (in bytes).
    pub checksum_size: u16,
    /// Size of the window.
    pub window_size: u16,
    /// Total size of the packet (including header and checksum part).
    pub packet_size: u16,
    /// Address to which answer.
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

    /// Check whether the `ack` number is within windows starting at `window_position` and specified by this connection.
    pub fn is_within_window(&self, ack: u16, window_position: u16, log: Box<&dyn Loggable>) -> bool {
        // get window borders
        let window_start = Wrapping(window_position);
        let window_end = window_start + Wrapping(self.window_size);
        // check if the window overlap over the range
        let is_within = match window_start < window_end {
            true => ack >= window_position && ack < window_position + self.window_size,
            false => window_position <= ack || ack < window_end.0,
        };
        // return the result
        log.vlog(&format!(
            "Check whether {} is within window starting at {} of size {}: {}",
            ack,
            window_position,
            self.window_size,
            is_within
        ));
        return is_within;
    }
}