use std::net::{UdpSocket, SocketAddr};
use std::fs::File;
use std::result::Result::Ok;
use std::io::Write;
use std::cmp::{max, min};

use rand::random;

use super::config::Config;
use crate::ConnectionProperties::ConnectionProperties;
use crate::packet::{InitPacket, Packet, ParsingError, Flag, EndPacket, PacketHeader, ToBin};

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

    let mut buffer = vec![0; connection_properties.packet_size as usize];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((read, from)) => {
                if config.is_verbose() {
                    println!("Received {}b from {}", read, from);
                }
                let packet = Packet::from_bin(&buffer[..read], connection_properties.checksum_size as usize);
                match packet {
                    Ok(packet) => {
                        let packet = check_packet_validity(packet, connection_properties.id, &from, &connection_properties.socket_addr)?;
                        match packet {
                            Packet::Init(packet) => {
                                send_init_packet_back(
                                    connection_properties.id,
                                    connection_properties.window_size,
                                    connection_properties.packet_size,
                                    connection_properties.checksum_size,
                                    &socket,
                                    &connection_properties.socket_addr
                                );
                            }
                            Packet::Error(packet) => {
                                return Err(String::from("Error packed received"));
                            }
                            Packet::End(packet) => {
                                if config.is_verbose() {
                                    println!("End packet received");
                                }
                                let answer = EndPacket::new(connection_properties.id, 0 /* TODO */);
                                let packet = Packet::from(answer);
                                let wrote = packet.to_bin_buff(&mut buffer, connection_properties.checksum_size as usize);
                                socket.send_to(&buffer[..wrote], connection_properties.socket_addr);
                                break;
                            }
                            Packet::Data(packet) => {

                            },
                        };
                    }
                    Err(ParsingError::InvalidSize(_, _)) if Flag::from_bin(&buffer[PacketHeader::flag_position()..]).unwrap() == Flag::Init => {
                        if config.is_verbose() {
                            println!("Received init packet again, sending connection info.")
                        }
                        send_init_packet_back(
                            connection_properties.id,
                            connection_properties.window_size,
                            connection_properties.packet_size,
                            connection_properties.checksum_size,
                            &socket,
                            &connection_properties.socket_addr
                        );
                    },
                    Err(e) => {
                        if config.is_verbose() {
                            println!("Error parsing packed: {:?}", e);
                        }
                        return Err(String::from("Error parsing packet"));
                    },
                };
            }
            Err(e) => {
                if config.is_verbose() {
                    println!("Error receiving packet: {}", e);
                }
                return Err(String::from("Error receiving packet"));
            }
        }
    }

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

fn check_packet_validity(packet: Packet, connection_id: u32, senderaddr: &SocketAddr, expectedsender: &SocketAddr) -> Result<Packet, String>{
    if expectedsender != senderaddr {
        return Err(format!("Expected to receive data from {}, but received from {}", expectedsender, senderaddr));
    }
    match packet {
        Packet::Init(packet) if packet.header.id != connection_id => {
            return Err(format!("Expected connection id {}, but received {}", connection_id, packet.header.id));
        }
        Packet::Error(packet) if packet.header.id != connection_id => {
            return Err(format!("Expected connection id {}, but received {}", connection_id, packet.header.id));
        }
        Packet::Data(packet) if packet.header.id != connection_id => {
            return Err(format!("Expected connection id {}, but received {}", connection_id, packet.header.id));
        }
        Packet::End(packet) if packet.header.id != connection_id => {
            return Err(format!("Expected connection id {}, but received {}", connection_id, packet.header.id));
        }
        _ => {}
    };


    return Ok(packet);
}


