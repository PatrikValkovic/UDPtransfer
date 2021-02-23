use std::net::{UdpSocket};
use std::fs::File;
use std::result::Result::Ok;
use std::io::ErrorKind;
use std::cmp::{max, min};
use std::collections::{HashMap as PropertiesMap};
use rand::Rng;
use itertools::Itertools;

use super::config::Config;
use crate::packet::{InitPacket, Packet, ParsingError, Flag, EndPacket, PacketHeader, ToBin, ErrorPacket};
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
            let filename = config.filename(prop.static_properties.id);
            let filepath = Path::new(&filename);
            if filepath.exists() {
                std::fs::remove_file(filepath).expect(&format!("Can't delete file for timeouted connection {}", conn_id));
            }
            if config.is_verbose() {
                println!("Connection {} closed because of timeout", conn_id);
            }
            let err_packet = Packet::from(ErrorPacket::new(conn_id));
            let bytes_to_write = err_packet.to_bin_buff(&mut buffer, prop.static_properties.checksum_size as usize);
            socket.send_to(&buffer[..bytes_to_write], prop.static_properties.socket_addr)
                .expect("Can't send error packet about the timeout");
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
            println!("Packet with flag {:?}", header.flag);
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

            Flag::Data => {}

            // Error flag
            Flag::Error => {}

            Flag::End => {}
        };
    };

    return Ok(());
}
