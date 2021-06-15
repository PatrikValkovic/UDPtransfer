use crate::connection_properties::ConnectionProperties;
use std::fs::File;
use std::net::UdpSocket;
use std::collections::BTreeMap;
use crate::sender::config::Config;
use std::time::{Instant, Duration};
use std::io::Read;
use crate::packet::{Packet, DataPacket, PacketHeader};
use std::num::Wrapping;
use std::cmp::min;

/// Part of the content that should be send.
struct Part {
    /// Actual content of the file.
    pub content: Vec<u8>,
    /// When this part was send for the last time.
    pub last_transition: Instant,
    /// Its sequence number.
    pub seq: u16,
    /// Whether the part was send (not necessarily received).
    pub send: bool,
}

/// Properties that the receiver stores per connection.
pub struct SenderConnectionProperties {
    /// Properties that the receiver and sender agreed on.
    pub static_properties: ConnectionProperties,
    /// Current position of the window. This number specified sequence number of the part the sender should send.
    pub window_position: u16,
    /// Cache memory of the parts sender should send.
    loaded_parts: BTreeMap<u16, Part>,
    /// Flag whether the sender read the whole file already.
    file_read: bool,
}

impl SenderConnectionProperties {
    pub fn new(props: ConnectionProperties) -> Self {
        Self {
            static_properties: props,
            window_position: 0,
            loaded_parts: BTreeMap::new(),
            file_read: false,
        }
    }

    /// Whether the whole file was send and confirmed.
    pub fn is_complete(&self) -> bool {
        return self.file_read && self.loaded_parts.len() == 0;
    }


    /// Check whether the `ack` number is within windows of this connection.
    fn is_within_window(&self, ack: u16, config: &Config) -> bool {
        self.static_properties.is_within_window(ack, self.window_position, Box::new(config))
    }

    /// Register acknowledge packet from the receiver with `ack` number.
    /// Return `true` if the window moved, false otherwise.
    pub fn acknowledge(&mut self, ack: u16, config: &Config) -> bool {
        config.vlog(&format!(
            "Acknowledge {} for connection {} with position {} and window size {}",
            ack,
            self.static_properties.id,
            self.window_position,
            self.static_properties.window_size
        ));
        // check if it is valid packet for current window
        if !self.is_within_window(ack, &config){
            return false;
        }
        // free cache memory for acknowledge packets
        let mut current_pos = Wrapping(self.window_position);
        let end_pos = Wrapping(ack) + Wrapping::<u16>(1);
        while current_pos != end_pos {
            self.loaded_parts.remove(&current_pos.0).expect("Can't remove entry for acknowledge");
            current_pos += Wrapping::<u16>(1);
        }
        // does the window moved?
        let moved = current_pos.0 != self.window_position;
        // move window if necessary.
        self.window_position = current_pos.0;
        // return value
        return moved;
    }

    /// Sends data over `socket` to the receiver of this connection.
    pub fn send_data(&mut self, socket: &UdpSocket, config: &Config){
        // create buffer
        let mut buffer = vec![0;self.static_properties.packet_size as usize];
        // for each part of the message
        for i in 0..min(self.static_properties.window_size, self.loaded_parts.len() as u16) {
            // get the part from the cache
            let current_index = Wrapping(self.window_position) + Wrapping(i);
            let part = self.loaded_parts.get_mut(&current_index.0).expect("Part is not within the map");
            // do not send if the timeout time doesn't exceed
            if part.send && Instant::now() - part.last_transition < Duration::from_millis(config.timeout as u64){
                continue;
            }
            config.vlog(&format!(
                "Connection {} will send data packet with seq {} and {}b of data",
                self.static_properties.id,
                part.seq,
                part.content.len()
            ));
            // create the packet for the part
            let data_packet = DataPacket::new(
                Clone::clone(&part.content),
                self.static_properties.id,
                part.seq,
                self.window_position,
            );
            // send the packet
            let response_size = Packet::from(data_packet).to_bin_buff(&mut buffer, self.static_properties.checksum_size as usize);
            socket.send_to(&buffer[..response_size], self.static_properties.socket_addr).expect("Can't send part of data");
            // update attributes of the part
            part.last_transition = Instant::now();
            part.send = true;
            config.vlog("Data packet send");
        }
    }

    /// Load content from the `file` to fill up the window.
    pub fn load_window(&mut self, file: &mut File, config: &Config){
        // if it read the whole file, do nothing
        if self.file_read {
            config.vlog("No more parts to read, as EOF occured");
            return;
        }

        // compute indices of parts to load
        let loaded_parts = Wrapping(self.loaded_parts.len() as u16);
        let mut load_index = Wrapping(self.window_position) + loaded_parts;
        let end_index = Wrapping(self.window_position) + Wrapping(self.static_properties.window_size);
        // decide how much data to load per packet
        let load_size = self.static_properties.packet_size - self.static_properties.checksum_size;
        let load_size = load_size as usize - PacketHeader::bin_size();
        config.vlog(&format!(
            "Connection {} has {} loaded parts, window size is {}, gonna be loaded {} parts, each of size {}",
            self.static_properties.id,
            loaded_parts.0,
            self.static_properties.window_size,
            self.static_properties.window_size - loaded_parts.0,
            load_size
        ));

        // load data
        let mut buffer = vec![0;load_size];
        while load_index != end_index {
            let read_size = file.read(buffer.as_mut_slice()).expect("Can't read file");
            config.vlog(&format!("Read {}b from file", read_size));
            if read_size == 0 { // if nothing read then it is end of the file
                self.file_read = true;
                break;
            }
            let part = Part {
                content: Vec::from(&buffer[..read_size]),
                last_transition: Instant::now(),
                seq: load_index.0,
                send: false,
            };
            config.vlog(&format!("Stored as part with seq {} and {}b of data", part.seq, part.content.len()));
            if let Some(_) = self.loaded_parts.insert(load_index.0, part){
                panic!("Part with this number os already loaded");
            }
            load_index += Wrapping::<u16>(1);
        }
    }
}