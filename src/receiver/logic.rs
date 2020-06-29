use std::net::{UdpSocket, SocketAddr};
use std::fs::File;
use std::result::Result::Ok;
use std::io::Write;

use super::config::Config;
use crate::ConnectionProperties::ConnectionProperties;
use rand::random;
use crate::packet::{InitPacket, Packet, ParsingError, Flag};
use std::cmp::{max, min};

pub fn logic(config: Config) -> Result<(), String> {
    let mut output_file = File::create(config.filename()).expect("Couldn't open file");
    if config.is_verbose() {
        println!("File {} opened", config.filename());
    }

    let socket = UdpSocket::bind(config.binding()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.binding());
    }

    let connection_properties = match connection_creation(&config, &socket) {
        Ok(prop) => prop,
        Err(()) => {
            println!("Can't create connection");
            return Err(String::from("Can't create connection"));
        }
    };

    /*
    let mut buff = Vec::new();
    buff.resize(config.max_packet_size() as usize, 0);

    while let Ok((size, sender)) = socket.recv_from(buff.as_mut_slice()) {
        if config.is_verbose() {
            println!("Received {}b of data from {}.", size, sender);
        }

        let wrote = output_file.write(&buff.as_slice()[..size]);
        match wrote {
            Ok(wrote) if config.is_verbose() => {
                println!("Wrote {}b of data", wrote);
            }
            Ok(_) => (),
            Err(e) => return Err(String::from("Error sending data")),
        };
    };
    */

    loop {

    };

    return Ok(());
}

fn connection_creation(config: &Config, socket: &UdpSocket) -> Result<ConnectionProperties, ()> {
    let connection_id: u32 = random();
    loop {
        let mut head = vec![0; 3000];
        let (read, from) = socket.peek_from(&mut head).unwrap();
        let packet = InitPacket::from_bin_noexcept(&head);
        if packet.header.flag != Flag::Init {
            if config.is_verbose() {
                println!("Received wrong packet type");
            }
            continue;
        }

        if config.is_verbose() {
            println!("Query packet size {}, window_size {}, checksum size: {}", packet.packet_size, packet.window_size, packet.checksum_size);
        }

        let mut head = vec![0; packet.packet_size as usize];
        let (read, from) = socket.recv_from(&mut head).unwrap();
        let parsed_packet = Packet::from_bin(&head[..read], packet.checksum_size as usize);
        match parsed_packet {
            Err(ParsingError::InvalidSize(expected, actual)) => {
                if config.is_verbose() {
                    println!("Not enough data received, expected {} received {}", expected, actual);
                }
                let new_packet_size = min(actual as u16, config.max_packet_size());
                let new_window_size = min(packet.window_size, config.max_window_size());
                send_init_packet_back(connection_id, new_window_size, new_packet_size, packet.checksum_size, &socket, &from);
                println!("Connection with {} created", from);
                if config.is_verbose() {
                    println!("Connection properties: packet size {}, window_size {}, checksum size: {}", new_packet_size, new_window_size, packet.checksum_size);
                }
                return Ok(ConnectionProperties::new(connection_id, packet.checksum_size, new_window_size, new_packet_size, from));
            }
            Ok(Packet::Init(packet)) => {
                let new_packet_size = min(packet.packet_size, config.max_packet_size());
                let new_window_size = min(packet.window_size, config.max_window_size());
                send_init_packet_back(connection_id, new_window_size, new_packet_size, packet.checksum_size, &socket, &from);
                println!("Connection with {} created", from);
                return Ok(ConnectionProperties::new(connection_id, packet.checksum_size, new_window_size, new_packet_size, from));
            }
            Ok(_) if config.is_verbose() => println!("Received unexpected packet type"),
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error receiving init packet: {:?}", e);
                return Err(());
            }
        }
    }
}

fn send_init_packet_back(connection_id: u32, window_size: u16, packet_size: u16, checksum_suze: u16, socket: &UdpSocket, addr: &SocketAddr) {
    let mut init = InitPacket::new(window_size, packet_size, checksum_suze);
    init.header.id = connection_id;
    let packet = Packet::Init(init);
    let buffer = packet.to_bin(checksum_suze as usize);
    socket.send_to(&buffer, addr);
}

