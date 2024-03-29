use std::net::{UdpSocket};
use std::result::Result::Ok;
use std::cmp::{max, min};
use std::collections::{HashMap as PropertiesMap};
use rand::Rng;
use itertools::Itertools;
use std::time::Duration;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::thread;
use super::config::Config;
use crate::packet::{InitPacket, Packet, ParsingError, Flag, EndPacket, PacketHeader, ToBin, ErrorPacket, DataPacket};
use crate::connection_properties::ConnectionProperties;
use crate::receiver::receiver_connection_properties::ReceiverConnectionProperties;
use crate::{BUFFER_SIZE, recv_with_timeout};


/// Creates the receiver.
/// `brk` parameter should be set to `true` when the receiver should terminate.
/// Returns handler to join the thread.
pub fn breakable_logic(config: Config, brk: Arc<AtomicBool>) -> JoinHandle<Result<(), String>> {
    thread::Builder::new()
        .name(String::from("Receiver"))
        .spawn(move || {
            receiver(config, brk)
        }).expect("Can't create thread for the broker")
}

/// Creates the receiver and keep running.
/// There is no way how to terminate the execution.
pub fn logic(config: Config) -> Result<(), String> {
    let brk = Arc::new(AtomicBool::new(false));
    receiver(config, brk)
}

fn receiver(config: Config, brk: Arc<AtomicBool>) -> Result<(), String> {
    // create socket
    let socket = UdpSocket::bind(config.binding()).expect("Can't bind socket");
    socket.set_read_timeout(Some(Duration::from_millis(config.timeout as u64))).expect("Can't set read timeout");
    config.vlog(&format!("Socket bind to {}", config.binding()));

    // create structures
    let mut random_generator = rand::thread_rng();
    let mut properties = PropertiesMap::<u32, ReceiverConnectionProperties>::new();
    let mut buffer = vec![0; BUFFER_SIZE];

    while !brk.load(Ordering::SeqCst) {
        // filter connections timeout
        // TODO use heap
        let ids_to_disconnect = properties.iter()
            .filter(|(_,prop)| prop.timeouted(config.timeout))
            .map(|(key,_)| *key)
            .collect_vec();
        for conn_id in ids_to_disconnect {
            let mut prop = properties.remove(&conn_id).expect("Connection is not in properties");
            remove_connection(&mut prop, &config, &mut buffer, &socket, "timeout");
        }
        // receive from socket
        let result = recv_with_timeout(&socket, &mut buffer, Box::new(&config));
        if let Err(_) = result {
            continue;
        }
        let (packet_size, received_from) = match result {
            Err(_) => continue,
            Ok(x) => x,
        };
        // get content
        config.vlog(&format!("Received packet of size {}", packet_size));
        let packet_content = &buffer[..packet_size];

        // parse header
        let header_result = PacketHeader::from_bin(packet_content);
        let header = match header_result {
            Err(e) => {
                if config.is_verbose() {
                    let header_in_bin = &buffer[..min(PacketHeader::bin_size(), packet_size)];
                    let header_in_str = Itertools::intersperse(
                        header_in_bin.iter().map(|num| { format!("{:02x}", num) }),
                        String::from("")
                    );
                    let header_in_str: String = header_in_str.collect();
                    config.vlog(&format!("Invalid header: {}; error: {:?}", header_in_str, e));
                }
                continue;
            }
            Ok(h) => h,
        };
        config.vlog(&format!("It is packet with flag {:?}", header.flag));

        // process init packet
        if let Flag::Init = header.flag {
            // Get content of init packet without checksum check, so it cat be used later
            // (and mainly infer what the checksum size should be)
            let init_content_result = InitPacket::from_bin_no_size_and_hash_check(&buffer[..packet_size]);
            let init_content = match init_content_result {
                Err(e) => {
                    config.vlog(&format!("Can't get content of init packet {:?}", e));
                    continue;
                }
                Ok(r) => r,
            };
            config.vlog(&format!(
                "Init packet properties, window size: {}, packet_size: {}, checksum: {}",
                init_content.window_size,
                init_content.packet_size,
                init_content.checksum_size
            ));
            // parse as packet
            let packet = Packet::from_bin(packet_content, init_content.checksum_size as usize);
            match packet {
                // everything OK, answer
                Ok(Packet::Init(_)) => {
                    // define properties
                    let window_size = min(init_content.window_size, config.max_window_size);
                    let packet_size = min(init_content.packet_size, config.max_packet_size);
                    let checksum_size = max(init_content.checksum_size, config.min_checksum);
                    let id: u32 = loop {
                        let id = random_generator.gen();
                        if !properties.contains_key(&id) && id > 0 {
                            break id;
                        }
                    };
                    // create connection properties
                    let props = ReceiverConnectionProperties::new(
                        ConnectionProperties::new(id, checksum_size, window_size, packet_size, received_from)
                    );
                    config.vlog(&format!(
                        "New connection {} with window_size: {}, packet_size: {}, checksum_size: {} created",
                        props.static_properties.id,
                        props.static_properties.window_size,
                        props.static_properties.packet_size,
                        props.static_properties.checksum_size,
                    ));
                    // store them
                    if let Some(_) = properties.insert(id, props) {
                        panic!("Connection with this ID already exists");
                    }
                    // answer the sender
                    let mut answer_packet = InitPacket::new(window_size, packet_size, checksum_size);
                    answer_packet.header.id = id;
                    let answer_length = Packet::from(answer_packet).to_bin_buff(&mut buffer, checksum_size as usize);
                    socket.send_to(&buffer[..answer_length], received_from).expect("Can't answer with init packet");
                    config.vlog("Answer init packet send");
                },
                // Not parsed init packet
                Ok(_) => {
                    config.vlog("Expected init packet, but parsed something different");
                }
                // Checksum not match, can't infer content
                Err(ParsingError::ChecksumNotMatch) => {
                    config.vlog("Checksum of init packet not match, ignoring");
                }
                // Received smaller packet, therefore checksum (and validity of data) can't be checked
                // Answer with receiver setting (and size that arrived) and let sender ask again
                Err(ParsingError::InvalidSize(expect, actual)) => {
                    config.vlog(&format!("Expected init packet of size {}, but received {}", expect, actual));
                    let return_init = InitPacket::new(
                        config.max_window_size,
                        min(config.max_packet_size, packet_size as u16),
                        config.min_checksum
                    );
                    config.vlog(&format!(
                        "Return init packet with properties, window size: {}, packet_size: {}, checksum: {}",
                        return_init.window_size,
                        return_init.packet_size,
                        return_init.checksum_size
                    ));
                    let answer_packet_size = Packet::from(return_init).to_bin_buff(buffer.as_mut_slice(), config.min_checksum as usize);
                    socket.send_to(&buffer[..answer_packet_size], received_from).expect("Can't answer with init packet after invalid size");
                    config.vlog("Return init packet send back");
                }
                // Other error
                Err(e) => {
                    config.vlog(&format!("Error parsing init packet {:?}", e));
                }
            };
            continue;
        }

        // validate connection id and get the properties of the connection
        let conn_id = header.id;
        let prop = match properties.get_mut(&conn_id) {
            Some(p) => p,
            None => {
                config.vlog(&format!("Received data packet for connection {}, but it doesn't exists", conn_id));
                continue;
            }
        };
        // parse packet if possible
        let packet = Packet::from_bin(&packet_content, prop.static_properties.checksum_size as usize);

        // process the flag
        match packet {
            Err(ParsingError::InvalidFlag(f)) => {
                config.vlog(&format!("Invalid flag {} received, ignoring packet", f));
            }
            Err(ParsingError::ChecksumNotMatch) => {
                config.vlog("Checksum does not match, ignoring");
            }
            Err(ParsingError::InvalidSize(exp, act)) => {
                config.vlog(&format!("Expected packet with size {}b, but only {}b received, ignoring", exp, act));
            }

            // data packet
            Ok(Packet::Data(packet)) => {
                config.vlog(&format!(
                    "Data packet for {} with seq {} and {}b of data, window at {} with size {}",
                    prop.static_properties.id,
                    packet.header.seq,
                    packet.data.len(),
                    prop.window_position,
                    prop.static_properties.window_size
                ));
                // make sure it is within window
                if !prop.is_within_window(packet.header.seq, &config) {
                    config.vlog("Data packed is not within window");
                }
                else {
                    // store it into structure
                    prop.store_data(&packet.data, packet.header.seq, &config);
                    // save it into file
                    prop.save_into_file(&config);
                }
                // return response
                let ack = prop.get_acknowledge();
                let packet = DataPacket::new_receiver(
                    prop.static_properties.id,
                    packet.header.seq,
                    ack
                );
                config.vlog(&format!("Answer with ack {}", packet.header.ack));
                let packet = Packet::from(packet);
                let response_size = packet.to_bin_buff(&mut buffer, prop.static_properties.checksum_size as usize);
                socket.send_to(&buffer[..response_size], received_from).expect("Can't respond to data packet");
                config.vlog("Answer data packet send");
            },

            // error packet
            Ok(Packet::Error(_)) => {
                let mut prop = properties.remove(&conn_id).expect("Can't remove connection property");
                remove_connection(&mut prop, &config, &mut buffer, &socket, "error packet");
                println!("Error received in connection {}", prop.static_properties.id);
            },

            // end packet
            Ok(Packet::End(packet)) => {
                if prop.parts_received.len() > 0 || prop.window_position != packet.header.seq {
                    config.vlog("Attempt to end packet, that has some blocks not stored");
                    let mut prop = properties.remove(&conn_id).expect("Can't remove connection properties for end packet with some data left");
                    remove_connection(&mut prop, &config, &mut buffer, &socket, "end packet with some data left");
                    continue;
                }
                prop.close();
                let response_packet = Packet::from(EndPacket::new(conn_id, prop.window_position));
                let response_length = response_packet.to_bin_buff(&mut buffer, prop.static_properties.checksum_size as usize);
                socket.send_to(&buffer[..response_length], received_from).expect("Can't send end packet");
                config.vlog(&format!("End of connection {}", prop.static_properties.id));
            },

            Ok(_) => {
                config.vlog("Received unexpected packet, ignoring");
            }
        }; // end of packet match
    }; // end of the main loop
    return Ok(());
} // end of the receiver method


fn remove_connection(
    prop: &mut ReceiverConnectionProperties,
    config: &Config,
    mut buffer: & mut Vec<u8>,
    socket: &UdpSocket,
    reason: &str,
) {
    // if the connection end successfully and now the structure is just deleted
    if prop.is_closed() {
        config.vlog(&format!("Connection {} definitely removed", prop.static_properties.id));
        return;
    }
    // delete the temp file
    prop.close();
    let filename = config.filename(prop.static_properties.id);
    let filepath = Path::new(&filename);
    if filepath.exists() {
        std::fs::remove_file(filepath).expect(&format!("Can't delete file for timeouted connection {}", prop.static_properties.id));
        config.vlog(&format!("Deleted file {}", filename));
    }
    // send back the error packet
    config.vlog(&format!("Connection {} closed because of {}", prop.static_properties.id, reason));
    let err_packet = Packet::from(ErrorPacket::new(prop.static_properties.id));
    let bytes_to_write = err_packet.to_bin_buff(&mut buffer, prop.static_properties.checksum_size as usize);
    socket.send_to(&buffer[..bytes_to_write], prop.static_properties.socket_addr)
        .expect(&format!("Can't send error packet about the {}", reason));
    config.vlog(&format!(
        "Error packet to {} with connection id {} send",
        prop.static_properties.socket_addr,
        prop.static_properties.id
    ));
}
