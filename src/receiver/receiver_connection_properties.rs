use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::num::Wrapping;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::connection_properties::ConnectionProperties;
use crate::receiver::config::Config;

pub struct ReceiverConnectionProperties {
    pub static_properties: ConnectionProperties,
    pub window_position: u16,
    pub next_write_position: u16,
    pub parts_received: BTreeMap<u16, Vec<u8>>,
    pub last_receive_time: Instant,
    is_closed: bool,
    file: Option<File>,
}

impl ReceiverConnectionProperties {
    pub fn new(conn_props: ConnectionProperties) -> Self {
        Self {
            static_properties: conn_props,
            next_write_position: 0,
            window_position: 0,
            parts_received: BTreeMap::new(),
            last_receive_time: Instant::now(),
            is_closed: false,
            file: None,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    pub fn close(&mut self) {
        self.is_closed = true;
        self.file.take();
    }

    pub fn timeouted(&self, timeout: u32) -> bool {
        let threshold_time = Instant::now() - Duration::from_millis(timeout as u64);
        return self.last_receive_time < threshold_time;
    }

    pub fn is_within_window(&self, ack: u16, config: &Config) -> bool {
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

    pub fn store_data(&mut self, data: &Vec<u8>,seq: u16, config: &Config) {
        // register data
        self.last_receive_time = Instant::now();
        // validate if data are within window
        if !self.is_within_window(seq, &config) {
            config.vlog("Not storing data, as they are outside of the window");
            return;
        }
        // store them
        self.parts_received.insert(seq, Clone::clone(data));
        config.vlog(&format!(
            "Connection {} stored {}b of data under seq {}",
            self.static_properties.id,
            data.len(),
            seq
        ));
        // move window if necessary
        while self.parts_received.contains_key(&self.window_position){
            let new_pos = Wrapping::<u16>(self.window_position) + Wrapping::<u16>(1);
            self.window_position = new_pos.0;
        }
        config.vlog(&format!(
            "Window moved to position {} for connection {}",
            self.window_position,
            self.static_properties.id
        ));
    }

    pub fn save_into_file(&mut self, config: &Config) {
        let path_str = config.filename(self.static_properties.id);
        let path = Path::new(&path_str);

        while self.next_write_position != self.window_position {
            let buffer = self.parts_received.remove(&self.next_write_position).expect("Part to write is not within the map");
            self.file = Some(match self.file.take() {
                Some(f) => f,
                None => OpenOptions::new().write(true)
                                          .append(true)
                                          .create(true)
                                          .open(path).expect("Can't open file for write")
            });
            let file = self.file.as_mut().unwrap();
            let wrote = file.write(&buffer).expect("Can't write to the output file");
            config.vlog(&format!(
               "Connection {} wrote {}b into file for packet seq {}",
                self.static_properties.id,
                wrote,
                self.next_write_position
            ));
            let new_write_pos = Wrapping(self.next_write_position) + Wrapping::<u16>(1);
            self.next_write_position = new_write_pos.0;
        }
    }

    pub fn get_acknowledge(&self) -> u16 {
        let ack = Wrapping(self.window_position) - Wrapping::<u16>(1);
        return ack.0;
    }
}