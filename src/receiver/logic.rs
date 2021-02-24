use std::net::{UdpSocket};
use std::result::Result::Ok;
use std::io::ErrorKind;
use std::cmp::{max, min};
use std::collections::{HashMap as PropertiesMap};
use rand::Rng;
use itertools::Itertools;

use super::config::Config;
use crate::packet::{InitPacket, Packet, ParsingError, Flag, EndPacket, PacketHeader, ToBin, ErrorPacket, DataPacket};
use crate::connection_properties::ConnectionProperties;
use crate::receiver::receiver_connection_properties::ReceiverConnectionProperties;
use std::time::Duration;
use std::path::Path;

pub fn logic(config: Config) -> Result<(), String> {
    let socket = UdpSocket::bind(config.binding()).expect("Can't bind socket");
    socket.set_read_timeout(Some(Duration::from_millis(config.get_timeout() as u64))).expect("Can't set read timeout");
    if config.is_verbose() {
        println!("Socket bind to {}", config.binding());
    }

    // create structures
    let mut random_generator = rand::thread_rng();
    let mut properties = PropertiesMap::<u32, ReceiverConnectionProperties>::new();

    let mut buffer = vec![0; 65535];
    loop {
        // filter timeouted connections
        // TODO use heap
        let ids_to_disconnect = properties.iter()
            .filter(|(_,prop)| prop.timeouted(config.get_timeout()))
            .map(|(key,_)| *key)
            .collect_vec();
        for conn_id in ids_to_disconnect {
            let prop = properties.remove(&conn_id).expect("Connection is not in properties");
            remove_connection(&prop, &config, &mut buffer, &socket, "timeout");
        }
        // receive from socket
        let result = socket.recv_from(&mut buffer);
        if let Err(e) = result {
            let kind = e.kind();
            if kind == ErrorKind::WouldBlock || kind == ErrorKind::TimedOut {
                continue;
            }
            if config.is_verbose() {
                println!("Error receiving packet, {:?}", e);
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
        if let Err(e) = header_result {
            if config.is_verbose() {
                let header_in_bin = &buffer[..PacketHeader::bin_size()];
                let header_in_str: String = header_in_bin.iter()
                    .map(|num| { format!("{:02x}", num) })
                    .intersperse(String::from(""))
                    .collect();
                println!("Invalid header: {}; error: {:?}",
                         header_in_str,
                         e
                );
            }
            continue;
        }
        let header = header_result.unwrap();
        if config.is_verbose() {
            println!("It is packet with flag {:?}", header.flag);
        }
        // process based on flag
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
                if let Err(ref e) = init_content_result {
                    if config.is_verbose() {
                        println!("Can't get content of init packet {:?}", e);
                        continue;
                    }
                }
                let init_content = init_content_result.unwrap();
                // parse as packet
                let packet = Packet::from_bin(packet_content, init_content.checksum_size as usize);
                match packet {
                    // everything OK, answer
                    Ok(Packet::Init(_)) => {
                        // define properties
                        let window_size = min(init_content.window_size, config.max_window_size());
                        let packet_size = min(init_content.packet_size, config.max_packet_size());
                        let checksum_size = max(init_content.checksum_size, config.min_checksum_size());
                        let id: u32 = loop {
                            let id = random_generator.gen();
                            if !properties.contains_key(&id) && id > 0 {
                                break id;
                            }
                        };
                        // create properties
                        let props = ReceiverConnectionProperties::new(
                            ConnectionProperties::new(id, checksum_size, window_size, packet_size, received_from)
                        );
                        if config.is_verbose() {
                            println!("New connection {} with window_size: {}, checksum_size: {}, packet_size: {} created",
                                     props.static_properties.id,
                                     props.static_properties.window_size,
                                     props.static_properties.checksum_size,
                                     props.static_properties.packet_size
                            );
                        }
                        // store them
                        let stored = properties.insert(id, props);
                        if let Some(_) = stored {
                            panic!("Connection with this ID already exists");
                        }
                        // answer the sender
                        let mut answer_packet = InitPacket::new(window_size, packet_size, checksum_size);
                        answer_packet.header.id = id;
                        let answer_length = Packet::from(answer_packet).to_bin_buff(&mut buffer, checksum_size as usize);
                        socket.send_to(&buffer[..answer_length], received_from).expect("Can't answer with init packet");
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
                        socket.send_to(&buffer[..answer_packet_size], received_from).expect("Can't answer with init packet after invalid size");
                        if config.is_verbose() {
                            println!("Send INIT packet back because of invalid size");
                        }
                    }
                    // Other error
                    Err(e) => {
                        if config.is_verbose() {
                            println!("Error parsing init packet {:?}", e);
                        }
                    }
                };
            }

            Flag::Data => {
                // get connection properties
                let conn_id = header.id;
                let prop = properties.get(&conn_id);
                if let None = prop {
                    if config.is_verbose() {
                        println!("Received data packet for connection {}, but it doesn't exists", conn_id);
                    }
                    continue;
                }
                let prop = prop.unwrap();
                // parse packet
                let packet = Packet::from_bin(&packet_content, prop.static_properties.checksum_size as usize);
                match packet {
                    Ok(Packet::Data(packet)) => {
                        // make sure it is within window
                        if !prop.is_withing_window(packet.header.seq) {
                            if config.is_verbose() {
                                println!("Data packed is not within window");
                            }
                            // TODO error if is too far away from the window
                            continue;
                        }
                        // store it into structure
                        let prop = properties.get_mut(&conn_id).unwrap();
                        prop.store_data(&packet.data, packet.header.seq);
                        // save it into file
                        prop.save_into_file(&config);
                        // return response
                        let ack = prop.get_acknowledge();
                        let packet = DataPacket::new_receiver(
                            prop.static_properties.id,
                            packet.header.seq,
                            ack
                        );
                        let packet = Packet::from(packet);
                        let response_size = packet.to_bin_buff(&mut buffer, prop.static_properties.checksum_size as usize);
                        socket.send_to(&buffer[..response_size], received_from).expect("Can't repond to data packet");
                    },
                    Ok(_) => {
                        if config.is_verbose() {
                            println!("Expected data packet but something different parsed");
                        }
                        continue;
                    }
                    Err(e) => {
                        if config.is_verbose() {
                            println!("Error parsing data packet {:?}", e);
                        }
                        continue;
                    }
                };
            }

            // Error flag
            Flag::Error => {
                // get connection properties
                let conn_id = header.id;
                let prop = properties.get(&conn_id);
                if let None = prop {
                    if config.is_verbose() {
                        println!("Received error packet for connection {}, but it doesn't exists", conn_id);
                    }
                    continue;
                }
                let prop = prop.unwrap();
                // get packet
                let packet = Packet::from_bin(&packet_content, prop.static_properties.checksum_size as usize);
                match packet {
                    Ok(Packet::Error(_)) => {
                        let prop = properties.remove(&conn_id).expect("Can't remove connection property");
                        remove_connection(&prop, &config, &mut buffer, &socket, "error packet");
                        println!("Error received in connection {}", prop.static_properties.id);
                    },
                    Ok(_) => {
                        if config.is_verbose() {
                            println!("Expected error packet but something different parsed");
                        }
                        continue;
                    }
                    Err(e) => {
                        if config.is_verbose() {
                            println!("Error parsing error packet {:?}", e);
                        }
                        continue;
                    }
                };
            }

            Flag::End => {
                // get connection properties
                let conn_id = header.id;
                let prop = properties.get(&conn_id);
                if let None = prop {
                    if config.is_verbose() {
                        println!("Received end packet for connection {}, but it doesn't exists", conn_id);
                    }
                    continue;
                }
                let prop = prop.unwrap();
                // get packet
                let packet = Packet::from_bin(&packet_content, prop.static_properties.checksum_size as usize);
                match packet {
                    Ok(Packet::End(packet)) => {
                        if prop.parts_received.len() > 0 || prop.window_position != packet.header.seq {
                            if config.is_verbose() {
                                println!("Attempt to end packet, that has some blocks not stored");
                            }
                            let prop = properties.remove(&conn_id).expect("Can't remove connection properties for end packet with some data left");
                            remove_connection(&prop, &config, &mut buffer, &socket, "end packet with some data left");
                            continue;
                        }
                        let prop = properties.remove(&conn_id).expect("Can't remove connection property");
                        let response_packet = Packet::from(EndPacket::new(conn_id, prop.window_position));
                        let response_length = response_packet.to_bin_buff(&mut buffer, prop.static_properties.checksum_size as usize);
                        socket.send_to(&buffer[..response_length], received_from).expect("Can't send end packet");
                        println!("End of connection {}", prop.static_properties.id);
                    },
                    Ok(_) => {
                        if config.is_verbose() {
                            println!("Expected end packet but something different parsed");
                        }
                        continue;
                    }
                    Err(e) => {
                        if config.is_verbose() {
                            println!("Error parsing end packet {:?}", e);
                        }
                        continue;
                    }
                };
            }
        };
    };
}

fn remove_connection(
    prop: &ReceiverConnectionProperties,
    config: &Config,
    mut buffer: & mut Vec<u8>,
    socket: &UdpSocket,
    reason: &str,
) {
    let filename = config.filename(prop.static_properties.id);
    let filepath = Path::new(&filename);
    if filepath.exists() {
        std::fs::remove_file(filepath).expect(&format!("Can't delete file for timeouted connection {}", prop.static_properties.id));
    }
    if config.is_verbose() {
        println!("Connection {} closed because of {}", prop.static_properties.id, reason);
    }
    let err_packet = Packet::from(ErrorPacket::new(prop.static_properties.id));
    let bytes_to_write = err_packet.to_bin_buff(&mut buffer, prop.static_properties.checksum_size as usize);
    socket.send_to(&buffer[..bytes_to_write], prop.static_properties.socket_addr)
        .expect(&format!("Can't send error packet about the {}", reason));
}
