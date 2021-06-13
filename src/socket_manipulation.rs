use std::net::{UdpSocket, SocketAddr};
use std::io::{ErrorKind, Result};
use crate::Loggable;

pub fn recv_with_timeout(
    socket: &UdpSocket,
    buff: &mut Vec<u8>,
    log: Box<&dyn Loggable>,
) -> Result<(usize, SocketAddr)> {
    // receive packet
    let result = socket.recv_from(buff.as_mut_slice());
    if let Err(e) = result {
        let kind = e.kind();
        if kind != ErrorKind::WouldBlock && kind != ErrorKind::TimedOut {
            log.vlog(&format!("Could not receive from socket {:?}, ignoring", socket.local_addr()));
            log.vlog(&format!("Error: {}", e.to_string()));
        }
        return Err(e);
    }
    return result;
}