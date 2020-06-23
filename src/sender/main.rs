mod config;

use std::net::{UdpSocket, SocketAddrV4};
use std::fs::File;
use std::result::Result::Ok;
use std::io::Read;

use config::Config;
use udp_transfer::packet::Packet;

fn main() -> std::io::Result<()> {
    let config = Config::from_command_line();

    let mut input_file = File::open(config.filename()).expect("Couldn't open file");
    if config.is_verbose() {
        println!("File {} opened", config.filename());
    }

    let socket = UdpSocket::bind(config.bind_addr()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.bind_addr());
    }
    let addr = config.send_addr();

    start_communication(&socket, &addr);

    let mut buff = Vec::new();
    buff.resize(config.max_packet_size() as usize, 0);
    let send_addr = config.send_addr();

    while let Ok(size) = input_file.read(buff.as_mut_slice()) {
        if config.is_verbose() {
            println!("Read {}b of data from file.", size);
        }

        if size == 0 {
            break;
        }

        let sent = socket.send_to(&buff.as_slice()[..size], send_addr)?;
        if config.is_verbose() {
            println!("Send {}b of data to {}.", sent, send_addr);
        }
    }

    return Ok(());
}

fn start_communication(socket: &UdpSocket, addr: &SocketAddrV4) {

}