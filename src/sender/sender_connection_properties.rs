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

struct Part {
    pub content: Vec<u8>,
    pub last_transition: Instant,
    pub seq: u16,
}

pub struct SenderConnectionProperties {
    pub static_properties: ConnectionProperties,
    pub window_position: u16,
    loaded_parts: BTreeMap<u16, Part>,
    read_whole: bool,
}

impl SenderConnectionProperties {
    pub fn new(props: ConnectionProperties) -> Self {
        Self {
            static_properties: props,
            window_position: 0,
            loaded_parts: BTreeMap::new(),
            read_whole: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        return self.read_whole && self.loaded_parts.len() == 0;
    }

    fn is_within_window(&self, ack: u16, config: &Config) -> bool {
        let window_start = Wrapping(self.window_position);
        let window_end = window_start + Wrapping(self.static_properties.window_size);
        let is_within: bool;
        if window_start < window_end {
            is_within = ack >= self.window_position && ack < self.window_position + self.static_properties.window_size;
        }
        else {
            is_within = self.window_position <= ack || ack < window_end.0;
        }
        config.vlog(&format!(
            "Check whether {} is within window starting at {} of size {}: {}",
            ack,
            self.window_position,
            self.static_properties.window_size,
            is_within
        ));
        return is_within;
    }

    pub fn acknowledge(&mut self, ack: u16, config: &Config) -> bool {
        config.vlog(&format!(
            "Acknowledge {} for connection {} with position {} and window size {}",
            ack,
            self.static_properties.id,
            self.window_position,
            self.static_properties.window_size
        ));

        if !self.is_within_window(ack, &config){
            return false;
        }

        let mut moved = false;
        let mut current_pos = Wrapping(self.window_position);
        let end_pos = Wrapping(ack) + Wrapping::<u16>(1);
        while current_pos != end_pos {
            self.loaded_parts.remove(&current_pos.0).expect("Can't remove entry for acknowledge");
            current_pos += Wrapping::<u16>(1);
            moved = true;
        }
        self.window_position = current_pos.0;
        return moved;
    }

    pub fn send_data(&mut self, socket: &UdpSocket, config: &Config){
        let mut buffer = vec![0;self.static_properties.packet_size as usize];
        for i in 0..min(self.static_properties.window_size, self.loaded_parts.len() as u16) {
            let current_index = Wrapping(self.window_position) + Wrapping(i);
            let part = self.loaded_parts.get_mut(&current_index.0).expect("Part is not within the map");
            if Instant::now() - part.last_transition < Duration::from_millis(config.timeout() as u64){
                continue;
            }
            config.vlog(&format!(
                "Connection {} will send data packet with seq {} and {}b of data",
                self.static_properties.id,
                part.seq,
                part.content.len()
            ));
            let data_packet = DataPacket::new(
                Clone::clone(&part.content),
                self.static_properties.id,
                part.seq,
                self.window_position,
            );
            let response_size = Packet::from(data_packet).to_bin_buff(&mut buffer, self.static_properties.checksum_size as usize);
            socket.send_to(&buffer[..response_size], self.static_properties.socket_addr).expect("Can't send part of data");
            part.last_transition = Instant::now();
            config.vlog("Data packet send");
        }
    }

    pub fn load_window(&mut self, file: &mut File, config: &Config){
        if self.read_whole {
            config.vlog("No more parts to read, as EOF occured");
            return;
        }

        let loaded_parts = Wrapping(self.loaded_parts.len() as u16);
        let mut load_index = Wrapping(self.window_position) + loaded_parts;
        let end_index = Wrapping(self.window_position) + Wrapping(self.static_properties.window_size);
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

        let mut buffer = vec![0;load_size];
        while load_index != end_index {
            let read_size = file.read(buffer.as_mut_slice()).expect("Can't read file");
            config.vlog(&format!("Read {}b from file", read_size));
            if read_size == 0 {
                self.read_whole = true;
                break;
            }
            let part = Part {
                content: Vec::from(&buffer[..read_size]),
                last_transition: Instant::now() - Duration::from_secs(60*60),
                seq: load_index.0
            };
            config.vlog(&format!("Stored as part with seq {} and {}b of data", part.seq, part.content.len()));
            if let Some(_) = self.loaded_parts.insert(load_index.0, part){
                panic!("Part with this number os already loaded");
            }
            load_index += Wrapping::<u16>(1);
        }
    }
}