use std::net::{UdpSocket, SocketAddr};
use std::fs::File;
use std::result::Result::Ok;
use std::io::Read;
use std::time::Duration;

use super::config::Config;
use super::sender_connection_properties::SenderConnectionProperties;
use crate::packet::{Packet, InitPacket, PacketHeader, ToBin, ParsingError};
use crate::connection_properties::ConnectionProperties;
use std::cmp::{min, max};

pub fn logic(config: Config) -> Result<(), String> {
    // open file
    let mut input_file = File::open(config.filename()).expect("Couldn't open file");
    if config.is_verbose() {
        println!("File {} opened", config.filename());
    }
    // connect socket
    let socket = UdpSocket::bind(config.bind_addr()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.bind_addr());
    }
    socket.set_read_timeout(Option::Some(Duration::from_millis(config.timeout() as u64))).expect("Can't set timeout on the socket");

    // init connection
    let props = create_connection(&config, &socket, config.send_addr()).expect("Can't create init connection");
    if config.is_verbose() {
        println!("Connection {} established, window_size: {}, packet_size: {}, checksum_size: {}",
                 props.static_properties.id,
                 props.static_properties.window_size,
                 props.static_properties.packet_size,
                 props.static_properties.checksum_size
        );
    }

    loop {

    }

    return Ok(());
}

fn create_connection(config: &Config, socket: &UdpSocket, addr: SocketAddr) -> Result<SenderConnectionProperties, ()> {
    let mut buffer = vec![0; 65535];
    // create my init packet
    let mut init_packet = InitPacket::new(
        config.window_size(),
        config.max_packet_size(),
        config.checksum_size()
    );

    for _ in 0..config.repetitions() {
        // send packet
        let packet = Packet::from(Clone::clone(&init_packet));
        let wrote = packet.to_bin_buff(&mut buffer, init_packet.checksum_size as usize);
        socket.send_to(&buffer[..wrote], addr).expect("Can't send data and establish init connection");
        if config.is_verbose() {
            println!("Init packet send - packet size: {}, checksum size: {}, window_size: {}", init_packet.packet_size, init_packet.checksum_size, init_packet.window_size)
        }
        // wait for answer
        let recv_result = socket.recv_from(&mut buffer);
        if let Err(e) = recv_result {
            if config.is_verbose() {
                println!("Can't receive data because of error {:?}", e);
            }
            continue;
        };
        // get raw data
        let (data_size, received_from) = recv_result.unwrap();
        if config.is_verbose() {
            println!("Received {} data from {}", data_size, received_from);
        }
        if data_size < PacketHeader::bin_size() {
            if config.is_verbose() {
                println!("Received less data than header, ignoring");
            }
            continue;
        }
        // parse init packet without exception
        let init_content_result = InitPacket::from_bin_no_size_and_hash_check(&buffer[..data_size]);
        if let Err(e) = init_content_result {
            if config.is_verbose() {
                println!("Can't read init content of the packet {:?}", e);
            }
            continue;
        }
        let init_content = init_content_result.unwrap();
        // parse packet itself
        let packet_result = Packet::from_bin(&buffer[..data_size], init_content.checksum_size as usize);
        match packet_result {
            Ok(Packet::Init(packet)) => {
                init_packet.packet_size = min(init_packet.packet_size, packet.packet_size);
                init_packet.window_size = min(init_packet.window_size, packet.window_size);
                init_packet.checksum_size = max(init_packet.checksum_size, packet.checksum_size);
                if packet.header.id == 0 {
                    if config.is_verbose() {
                        println!("Received init packet with 0 id, receiver couldn't receive whole packet, repeating");
                    }
                    continue;
                }
                return Ok(SenderConnectionProperties::new(
                    ConnectionProperties::new(
                        packet.header.id,
                        init_packet.checksum_size,
                        init_packet.window_size,
                        init_packet.packet_size,
                        received_from
                    )
                ));
            }
            Ok(_) => {
                if config.is_verbose() {
                    println!("Not init packet received, dropping");
                }
            }
            Err(ParsingError::InvalidSize(expected, actual)) => {
                init_packet.packet_size = actual as u16;
                if config.is_verbose() {
                    println!("Expected to received {} bytes, but {} only received, repeating with new configuration", expected, actual);
                }
                continue;
            }
            Err(e) => {
                println!("Packet can't be parsed: {:?}", e);
                return Err(());
            }
        };
    }

    println!("Can't establish connection with the server after {} attempts", config.repetitions());
    return Err(());
}