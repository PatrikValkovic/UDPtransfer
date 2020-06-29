use std::net::{UdpSocket, SocketAddrV4, SocketAddr};
use std::fs::File;
use std::result::Result::Ok;
use std::io::{Read, ErrorKind};
use std::time::Duration;

use super::config::Config;
use crate::ConnectionProperties::ConnectionProperties;
use crate::packet::{Packet, InitPacket};

pub fn logic(config: Config) -> Result<(), String> {
    let mut input_file = File::open(config.filename()).expect("Couldn't open file");
    if config.is_verbose() {
        println!("File {} opened", config.filename());
    }

    let socket = UdpSocket::bind(config.bind_addr()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.bind_addr());
    }
    if let Err(e) = socket.set_read_timeout(Option::Some(Duration::from_millis(config.timeout() as u64))) {
        if config.is_verbose() {
            println!("Can't set read timeout on the socket");
        }
        return Err(String::from("Can't set read timeout on the socket"));
    }

    let props = match create_connection(&config, &socket, config.send_addr()) {
        Ok(props) => props,
        Err(()) => {
            println!("Can't create connection");
            return Err(String::from("Can't create connection"));
        }
    };

    loop {

    }

    /*
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

        let sent = socket.send_to(&buff.as_slice()[..size], send_addr);
        match sent {
            Ok(sent) if config.is_verbose() => {
                println!("Send {}b of data to {}.", sent, send_addr);
            },
            Ok(_) => (),
            Err(e) => return Err(String::from("Error sending data")),
        };
    }
    */

    return Ok(());
}

fn create_connection(config: &Config, socket: &UdpSocket, addr: SocketAddr) -> Result<ConnectionProperties, ()> {
    let mut buffer = vec![0; config.max_packet_size() as usize];
    for _ in 0..config.repetitions() {
        let packet = Packet::from(InitPacket::new(config.window_size(), config.max_packet_size(), config.checksum_size()));
        let wrote = packet.to_bin_buff(&mut buffer, config.checksum_size() as usize);
        socket.send_to(&buffer[..wrote], addr);
        if let Packet::Init(packet) = packet {
            if config.is_verbose() {
                println!("Init packet send - packet size: {}, checksum size: {}, window_size: {}", packet.packet_size, packet.checksum_size, packet.window_size)
            }
        }

        match socket.recv_from(&mut buffer) {
            Ok((recv, _)) => {
                let packet = Packet::from_bin(&buffer[..recv], config.checksum_size() as usize);
                match packet {
                    Ok(Packet::Init(packet)) => {
                        println!("Connection established");
                        if config.is_verbose() {
                            println!("Connection properties - packet size: {}, checksum size: {}, window_size: {}", packet.packet_size, packet.checksum_size, packet.window_size)
                        }
                        return Ok(ConnectionProperties::new(
                            packet.header.id,
                            packet.checksum_size,
                            packet.window_size,
                            packet.packet_size,
                            addr
                        ));
                    }
                    Ok(_) => {
                        if config.is_verbose() {
                            println!("Invalid packet received, dropping");
                        }
                    }
                    Err(e) => {
                        println!("Packet can't be parsed: {:?}", e);
                        return Err(());
                    }
                }
            },
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                if config.is_verbose() {
                    println!("Init recv timeouted, repeating");
                }
            }
            Err(e) => {
                if config.is_verbose() {
                    println!("Error receiving data: {:?}", e);
                }
                return Err(());
            }
        };
    }
    println!("Can't establish connection with the server");
    return Err(());
}