use std::cmp::{max, min};
use std::fs::File;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::result::Result::Ok;
use std::time::Duration;

use crate::connection_properties::ConnectionProperties;
use crate::packet::{EndPacket, ErrorPacket, InitPacket, Packet, PacketHeader, ParsingError, Flag};

use super::config::Config;
use super::sender_connection_properties::SenderConnectionProperties;
use crate::{recv_with_timeout, BUFFER_SIZE};

pub fn logic(config: Config) -> Result<(), String> {
    // open file
    let mut input_file = File::open(config.filename()).expect("Couldn't open file");
    config.vlog(&format!("File {} opened", config.filename()));
    // connect socket
    let socket = UdpSocket::bind(config.bind_addr()).expect("Can't bind socket");
    config.vlog(&format!("Socket bind to {}", config.bind_addr()));
    socket.set_read_timeout(Option::Some(Duration::from_millis(config.timeout() as u64))).expect("Can't set timeout on the socket");
    // init connection
    let props = create_connection(&config, &socket, config.send_addr()).expect("Can't create init connection");
    let mut props = SenderConnectionProperties::new(
      props,
    );
    config.vlog(&format!("Connection {} established, window_size: {}, packet_size: {}, checksum_size: {}",
                         props.static_properties.id,
                         props.static_properties.window_size,
                         props.static_properties.packet_size,
                         props.static_properties.checksum_size));

    // send data
    let send = send_data(&config, &mut input_file, &socket, &mut props)?;

    send_end(config, socket, props)
}

/// Connect to the receiver and agree on the connection properties.
/// It uses `socket` and expect receiver at the `addr` address.
fn create_connection(config: &Config, socket: &UdpSocket, addr: SocketAddr) -> Result<ConnectionProperties, ()> {
    // create buffer
    let mut buffer = vec![0; BUFFER_SIZE];
    // create my init packet
    let mut init_packet = InitPacket::new(
        config.window_size(),
        config.max_packet_size(),
        config.checksum_size()
    );

    // for specified number of retries
    for a in 0..config.repetitions() {
        // send packet
        config.vlog(&format!("Attempt {} to establish connection", a+1));
        let packet = Packet::from(Clone::clone(&init_packet));
        let wrote = packet.to_bin_buff(&mut buffer, init_packet.checksum_size as usize);
        socket.send_to(&buffer[..wrote], addr).expect("Can't send data and establish connection");
        config.vlog(&format!(
            "Init packet send - packet size: {}, checksum size: {}, window_size: {}",
            init_packet.packet_size,
            init_packet.checksum_size,
            init_packet.window_size
        ));
        // wait for answer
        let recv_result = recv_with_timeout(&socket, &mut buffer, Box::new(config));
        if let Err(_) = recv_result {
            continue;
        };
        // get raw data
        let (data_size, received_from) = recv_result.unwrap();
        config.vlog(&format!("Received {} data from {}", data_size, received_from));
        if data_size < PacketHeader::bin_size() {
            config.vlog("Received less data than header, ignoring");
            continue;
        }
        // parse init packet without exception
        let init_content_result = InitPacket::from_bin_no_size_and_hash_check(&buffer[..data_size]);
        if let Err(e) = init_content_result {
            config.vlog(&format!("Can't read init content of the packet {:?}", e));
            continue;
        }
        let init_content = init_content_result.unwrap();
        // parse packet itself
        let packet_result = Packet::from_bin(&buffer[..data_size], init_content.checksum_size as usize);
        // decide what to do with the packet
        match packet_result {
            Ok(Packet::Init(packet)) => {
                init_packet.packet_size = min(init_packet.packet_size, packet.packet_size);
                init_packet.window_size = min(init_packet.window_size, packet.window_size);
                init_packet.checksum_size = max(init_packet.checksum_size, packet.checksum_size);
                if packet.header.id == 0 {
                    config.vlog("Received init packet with 0 id, receiver couldn't receive whole packet, repeating");
                    continue;
                }
                return Ok(ConnectionProperties::new(
                        packet.header.id,
                        init_packet.checksum_size,
                        init_packet.window_size,
                        init_packet.packet_size,
                        received_from
                    ));
            }
            Ok(_) => {
                config.vlog("Not init packet received, dropping");
            }
            Err(ParsingError::InvalidSize(expected, actual)) => {
                init_packet.packet_size = actual as u16;
                config.vlog(&format!("Expected to received {} bytes, but {} only received, repeating with new configuration", expected, actual));
                continue;
            }
            Err(e) => {
                config.vlog(&format!("Packet can't be parsed: {:?}", e));
                return Err(());
            }
        };
    }
    // didn't receive init packet after specified number of retries
    println!("Can't establish connection with the server after {} attempts", config.repetitions());
    return Err(());
}


/// Send the data after connection has been established.
/// It send `input_file` file via `socket` using the `props` connection.
fn send_data(
    config: &Config,
    mut input_file: &mut File,
    socket: &UdpSocket,
    mut props: &mut SenderConnectionProperties
) -> Result<(), String> {
    // load
    props.load_window(&mut input_file, &config);
    // prepare variables
    let mut attempts = 0;
    let mut buffer = vec![0; BUFFER_SIZE];
    // process data
    while attempts < config.repetitions() && !props.is_complete() {
        // send content from the window
        props.load_window(&mut input_file, &config);
        props.send_data(&socket, &config);
        // receive response
        let content_result = socket.recv_from(&mut buffer);
        // process errors for receive
        if let Err(e) = content_result {
            let kind = e.kind();
            if kind == ErrorKind::TimedOut || kind == ErrorKind::WouldBlock {
                config.vlog("Recv timeout");
                attempts += 1;
                config.vlog(&format!("Increased number of attempts to {}", attempts));
                continue;
            }
            config.vlog(&format!("Recv error: {:?}", e));
            return Err(String::from("Can't receive data"));
        }
        let (recived_len, recived_from) = content_result.unwrap();
        config.vlog(&format!("Received {}b of data from {}", recived_len, recived_from));
        // read content and validate it
        let packet = Packet::from_bin(&buffer[..recived_len], props.static_properties.checksum_size as usize);
        let packet = match packet {
            Err(ParsingError::ChecksumNotMatch) => {
                config.vlog("Invalid sum, ignoring");
                continue;
            }
            Err(ParsingError::InvalidFlag(f)) => {
                config.vlog(&format!("Invalid flag {}, ignoring", f));
                continue;
            },
            Err(ParsingError::InvalidSize(expected, actual)) => {
                config.vlog(&format!("Expected {}b but received {}b, ignoring", expected, actual));
                continue;
            },
            Ok(packet) => {
                if packet.header().id != props.static_properties.id {
                    config.vlog("Wrong connection ID");
                    continue;
                }
                packet
            }
        };
        match packet {
            Packet::Init(_) => {
                config.vlog("Init packet received, but connection already established");
                continue;
            }
            Packet::End(_) => {
                config.vlog("End packet received, but hasn't been expected");
                let error_packet = ErrorPacket::new(props.static_properties.id);
                let answer_length = Packet::from(error_packet).to_bin_buff(&mut buffer, props.static_properties.checksum_size as usize);
                socket.send_to(&buffer[..answer_length], config.send_addr()).expect("Can't send error packet");
                return Err(String::from("Unexpected end packet"));
            },
            Packet::Error(_) => {
                config.vlog("Error packet received");
                println!("Failed because error packet received");
                return Err(String::from("Error packet received"));
            },
            Packet::Data(packet) => {
                if props.acknowledge(packet.header.ack, &config) {
                    attempts = 0;
                }
            }
        };
    };

    if attempts == config.repetitions() {
        config.vlog(&format!("Connection lost after {} attempts", attempts));
        return Err(format!("Connection lost after {} attemps", attempts));
    }
    config.vlog("All data send");
    return Ok(());
}

/// Ends the connection after the file has been received.
/// It sends data using `socket` and closes connection specified by `props`.
fn send_end(config: Config, socket: UdpSocket, mut props: SenderConnectionProperties) -> Result<(), String> {
    // creates variables
    let mut buffer = vec![0; BUFFER_SIZE];
    let packet = Packet::from(EndPacket::new(
        props.static_properties.id,
        props.window_position,
    ));
    // wait for end packet
    let mut attempts = 0;
    while attempts < config.repetitions() {
        // send end packet
        let size = packet.to_bin_buff(&mut buffer, props.static_properties.checksum_size as usize);
        socket.send_to(&buffer[..size], props.static_properties.socket_addr).expect("Can't send end packet");
        config.vlog("Send end packet");
        // receive response
        let recv_result = recv_with_timeout(&socket, &mut buffer, Box::new(&config));
        if let Err(_) = recv_result {
            attempts += 1;
            continue;
        }
        let (recv_size, _) = recv_result.unwrap();
        // parse packet
        let packet = Packet::from_bin(&buffer[..recv_size], props.static_properties.checksum_size as usize);
        if let Err(e) = packet {
            config.vlog(&format!("Error parsing end packet {:?}", e));
            continue;
        }
        let packet = packet.unwrap();
        // make sure it is packet for this connection
        if packet.header().id != props.static_properties.id {
            config.vlog("Receive packet with invalid connection number");
            if Flag::Init == packet.header().flag {
                continue; // init flag delay on the way with not established connection
            }
            return Err(String::from("Received packet with invalid connection number"));
        }
        // handle end packet
        match packet {
            // it is end packet as expected
            Packet::End(packet) => {
                // if the end packet is invalid send error and terminate
                if packet.header.ack != props.window_position || packet.header.seq != props.window_position {
                    config.vlog("Received invalid end packet");
                    let error_packet = ErrorPacket::new(props.static_properties.id);
                    let answer_length = Packet::from(error_packet).to_bin_buff(&mut buffer, props.static_properties.checksum_size as usize);
                    socket.send_to(&buffer[..answer_length], config.send_addr()).expect("Can't send error packet");
                    return Err(String::from("Invalid end packet"));
                }
                // else end peacefully
                println!("File receive confirmed");
                return Ok(());
            },
            // error on the receiver part, ending
            Packet::Error(_) => {
                config.vlog("Received error packet instead of end packet");
                return Err(String::from("Error packet received"));
            },
            // data or init packet delayed on the way, ignoring
            _ => {
                config.vlog("Received data or init packet after sending end packet, ignoring");
                continue;
            }
        };
    }
    return Err(String::from("End packet timeout"));
}

