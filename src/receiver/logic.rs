use std::net::{UdpSocket, SocketAddr};
use std::fs::File;
use std::result::Result::Ok;
use std::io::Write;
use std::cmp::{max, min};
use itertools::Itertools;
use rand::random;

use super::config::Config;
use crate::ConnectionProperties::ConnectionProperties;
use crate::packet::{InitPacket, Packet, ParsingError, Flag, EndPacket, PacketHeader, ToBin};

pub fn logic(config: Config) -> Result<(), String> {
    let socket = UdpSocket::bind(config.binding()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.binding());
    }

    let mut buffer = vec![0; 65535];
    while true{
        let result = socket.recv_from(&mut buffer);
        if let Err(E) = result {
            if config.is_verbose(){
                println!("Error receiving packet, {:?}", E);
            }
            continue;
        };

        let (packet_size, received_from) = result.into_ok();
        if packet_size < PacketHeader::bin_size() {
            if config.is_verbose(){
                println!("Invalid packet with size {}", packet_size);
            }
            continue;
        }

        let header_result = PacketHeader::from_bin(&buffer[..PacketHeader::bin_size()]);
        if let Err(E) = header_result {
            if config.is_verbose() {
                let header_in_bin = buffer[..PacketHeader::bin_size()];
                let header_in_str = header_in_bin.iter()
                    .map(|num| {format!("{:02x}", num)})
                    .intersperse(String::from(""))
                    .collect();
                println!("Invalid header: {:?}; raw: {}",
                         E,
                         header_in_str
                );
            }
            continue;
        }
        let header = header_result.into_ok();

        match header.flag {
            Flag::None => {}
            Flag::Init => {}
            Flag::Data => {}
            Flag::Error => {}
            Flag::End => {}
        };
    };

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

fn send_init_packet_back(connection_id: u32, window_size: u16, packet_size: u16, checksum_suze: u16, socket: &UdpSocket, addr: &SocketAddr) {
    let mut init = InitPacket::new(window_size, packet_size, checksum_suze);
    init.header.id = connection_id;
    let packet = Packet::Init(init);
    let buffer = packet.to_bin(checksum_suze as usize);
    socket.send_to(&buffer, addr);
}


