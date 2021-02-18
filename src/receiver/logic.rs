use std::net::{UdpSocket, SocketAddr};
use std::fs::File;
use std::result::Result::Ok;
use std::io::Write;
use std::cmp::{max, min};
use itertools::Itertools;
use rand::random;

use super::config::Config;
use crate::connection_properties::ConnectionProperties;
use crate::packet::{InitPacket, Packet, ParsingError, Flag, EndPacket, PacketHeader, ToBin};

pub fn logic(config: Config) -> Result<(), String> {
    let socket = UdpSocket::bind(config.binding()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.binding());
    }

    let mut buffer = vec![0; 65535];
    loop {
        // receive from socket
        let result = socket.recv_from(&mut buffer);
        if let Err(E) = result {
            if config.is_verbose() {
                println!("Error receiving packet, {:?}", E);
            }
            continue;
        };
        // get content
        let (packet_size, received_from) = result.unwrap();
        if packet_size < PacketHeader::bin_size() {
            if config.is_verbose() {
                println!("Invalid packet with size {}", packet_size);
            }
            continue;
        }
        if config.is_verbose() {
            println!("Received packet of size {}", packet_size);
        }
        let packet_content = &buffer[..packet_size];
        // parse header
        let header_result = PacketHeader::from_bin(&buffer[..PacketHeader::bin_size()]);
        if let Err(E) = header_result {
            if config.is_verbose() {
                let header_in_bin = &buffer[..PacketHeader::bin_size()];
                let header_in_str: String = header_in_bin.iter()
                    .map(|num| { format!("{:02x}", num) })
                    .intersperse(String::from(""))
                    .collect();
                println!("Invalid header: {}; error: {:?}",
                         header_in_str,
                         E
                );
            }
            continue;
        }
        let header = header_result.unwrap();
        if config.is_verbose() {
            println!("Packet with flag {:?}", header.flag);
        }
        //process based on flag
        match header.flag {

            // None flag ignore
            Flag::None => {
                if config.is_verbose() {
                    println!("Flag is not specified");
                }
                continue;
            }

            // Init flag
            Flag::Init => {
                // Get content of init packet
                let init_content_result = InitPacket::from_bin_no_size_and_hash_check(&buffer[..packet_size]);
                if let Err(ref E) = init_content_result {
                    if config.is_verbose() {
                        println!("Can't get content of init packet {:?}", E);
                        continue;
                    }
                }
                let init_content = init_content_result.unwrap();
                // parse as packet
                let packet = Packet::from_bin(packet_content, init_content.checksum_size as usize);
                match packet {
                    // everything OK, answer
                    Ok(Packet::Init(InitPacket)) => {
                        //TODO
                    },
                    // Not parsed init packet
                    Ok(_) => {
                        if config.is_verbose() {
                            println!("Expected init packet, but parsed something different");
                        }
                    }
                    // Checksum not match, can't infer content
                    Err(ParsingError::ChecksumNotMatch) => {
                        if config.is_verbose() {
                            println!("Checksum of init packet not match, ignoring");
                        }
                    }
                    // Received smaller packet, therefore checksum (and validity of data) can't be checked
                    // Answer with receiver setting and let sender ask once again
                    Err(ParsingError::InvalidSize(expect, actual)) => {
                        if config.is_verbose() {
                            println!("Expected init packet of size {}, but received {}", expect, actual);
                        }
                        let return_init = Packet::from(InitPacket::new(
                            config.max_window_size(),
                            min(config.max_packet_size(), packet_size as u16),
                            config.min_checksum_size()
                        ));
                        let answer_packet_size = return_init.to_bin_buff(buffer.as_mut_slice(), config.min_checksum_size() as usize);
                        socket.send_to(&buffer[..answer_packet_size], received_from);
                        if config.is_verbose() {
                            println!("Send INIT packet back because of invalid size");
                        }
                    }
                    // Other error
                    Err(E) => {
                        if config.is_verbose() {
                            println!("Error parsing init packet {:?}", E);
                        }
                    }
                };
            }

            Flag::Data => {}
            Flag::Error => {}
            Flag::End => {
                break;
            }
        };
    };

    return Ok(());

    /*let connection_properties = match connection_creation(&config, &socket) {
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
     */
}

/*
fn send_init_packet_back(connection_id: u32, window_size: u16, packet_size: u16, checksum_suze: u16, socket: &UdpSocket, addr: &SocketAddr) {
    let mut init = InitPacket::new(window_size, packet_size, checksum_suze);
    init.header.id = connection_id;
    let packet = Packet::Init(init);
    let buffer = packet.to_bin(checksum_suze as usize);
    socket.send_to(&buffer, addr);
}
*/

