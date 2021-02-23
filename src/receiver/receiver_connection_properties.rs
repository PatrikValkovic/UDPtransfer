use std::collections::BTreeSet;
use std::time::{Instant, Duration};
use crate::connection_properties::ConnectionProperties;
use argparse::List;
use crate::receiver::config::Config;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::Write;

pub struct ReceiverConnectionProperties {
    pub static_properties: ConnectionProperties,
    pub window_position: u16,
    pub window_buffer: Vec<u8>,
    pub parts_received: BTreeSet<u16>,
    pub last_receive_time: Instant,
    file: Option<File>,
}

impl ReceiverConnectionProperties {
    pub fn new(conn_props: ConnectionProperties) -> Self {
        let buffer_length = (conn_props.window_size * (conn_props.packet_size - conn_props.checksum_size)) as usize;
        Self {
            static_properties: conn_props,
            window_position: 0,
            window_buffer: vec![0; buffer_length],
            parts_received: BTreeSet::new(),
            last_receive_time: Instant::now(),
            file: None,
        }
    }

    pub fn timeouted(&self, timeout: u32) -> bool {
        let threshold_time = Instant::now() - Duration::from_millis(timeout as u64);
        return self.last_receive_time < threshold_time;
    }

    pub fn is_withing_window(&self, seq: u16) -> bool {
        if self.window_position + self.static_properties.window_size < self.window_position {
            return seq >= self.window_position || seq < self.window_position + self.static_properties.window_size;
        }
        else {
            return self.window_position <= seq && seq < self.window_position + self.static_properties.window_size;
        }
    }

    fn position_in_window(&self, seq: u16) -> u16 {
        if seq >= self.window_position {
            return seq - self.window_position;
        }
        else {
            return self.static_properties.packet_size - self.window_position + seq;
        }
    }

    pub fn store_data(&mut self, data: &Vec<u8>,seq: u16) {
        let pos_in_window = self.position_in_window(seq);
        self.parts_received.insert(pos_in_window);
        let data_length = self.static_properties.packet_size - self.static_properties.checksum_size;
        let data_length = data_length as usize;
        let pos_in_window = pos_in_window as usize;
        let buffer_storage = &mut self.window_buffer[pos_in_window*data_length..(pos_in_window+1)*data_length];
        buffer_storage.copy_from_slice(data.as_slice());
    }

    pub fn save_into_file(&mut self, config: &Config) {
        let path = config.filename(self.static_properties.id);
        let path = Path::new(&path);
        let data_length = self.static_properties.packet_size - self.static_properties.checksum_size;
        let data_length = data_length as usize;
        while self.parts_received.contains(&self.window_position) {
            let mut file = OpenOptions::new().write(true).append(true).open(path).expect("Can't open file for write");
            file.write(&self.window_buffer[..data_length]).expect("Can't write to the file");
            if !self.parts_received.remove(&self.window_position){
                panic!("Can't remove packet content from the window tree");
            };
            for i in 1..self.static_properties.window_size as usize {
                self.window_buffer.copy_within(
                    i*data_length..(i+1)*data_length,
                    (i-1)*data_length
                );
            }
            self.window_position += 1;
        }
    }

    pub fn get_acknowledge(&self) -> u16 {
        return self.window_position;
    }
}