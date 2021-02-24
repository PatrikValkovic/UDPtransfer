use crate::connection_properties::ConnectionProperties;
use std::fs::File;
use std::net::UdpSocket;
use std::collections::BTreeMap;
use crate::sender::config::Config;
use std::time::{Instant, Duration};
use std::io::Read;
use crate::packet::{Packet, DataPacket, PacketHeader};

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

    fn is_within_window(&self, ack: u16) -> bool {
        if self.window_position + self.static_properties.window_size < self.window_position {
            return ack >= self.window_position || ack < self.window_position + self.static_properties.window_size;
        }
        else {
            return self.window_position <= ack && ack < self.window_position + self.static_properties.window_size;
        }
    }

    pub fn acknowledge(&mut self, ack: u16) {
        if !self.is_within_window(ack){
            return;
        }

        while self.window_position != ack {
            self.loaded_parts.remove(&self.window_position).expect("Can't remove entry for acknowledge");
            self.window_position += 1;
        }
    }

    pub fn send_data(&mut self, socket: &UdpSocket, config: &Config){
        let mut buffer = vec![0;self.static_properties.packet_size as usize];
        for i in 0..self.static_properties.window_size {
            let current_index = self.window_position + i;
            let part = self.loaded_parts.get_mut(&current_index).expect("Part is not within the map");
            if Instant::now() - part.last_transition < Duration::from_millis(config.timeout() as u64){
                continue;
            }
            let data_packet = DataPacket::new(
                Clone::clone(&part.content),
                self.static_properties.id,
                part.seq,
                self.window_position,
            );
            let response_size = Packet::from(data_packet).to_bin_buff(&mut buffer, self.static_properties.checksum_size as usize);
            socket.send_to(&buffer[..response_size], self.static_properties.socket_addr).expect("Can't send part of data");
        }
    }

    pub fn load_window(&mut self, file: &mut File){
        let loaded_parts = self.loaded_parts.len() as u16;
        let mut load_index = self.window_position + loaded_parts;
        let end_index = self.window_position + self.static_properties.window_size;
        let load_size = self.static_properties.packet_size - self.static_properties.checksum_size;
        let load_size = load_size as usize - PacketHeader::bin_size();
        let mut buffer = vec![0;load_size];
        while load_index != end_index {
            let read_size = file.read(buffer.as_mut_slice()).expect("Can't read file");
            if read_size == 0 {
                break;
            }
            let part = Part {
                content: Vec::from(&buffer[..read_size]),
                last_transition: Instant::now() - Duration::from_secs(60*60),
                seq: load_index
            };
            if let Some(_) = self.loaded_parts.insert(load_index, part){
                panic!("Part with this number os already loaded");
            }
            load_index += 1;
        }
    }
}