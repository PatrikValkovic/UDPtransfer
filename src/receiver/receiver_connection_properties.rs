use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::num::Wrapping;
use std::path::Path;
use std::time::{Duration, Instant};
use crate::connection_properties::ConnectionProperties;
use crate::receiver::config::Config;

/// Properties that the receiver stores per connection.
pub struct ReceiverConnectionProperties {
    /// Properties that the receiver and sender agreed on.
    pub static_properties: ConnectionProperties,
    /// Current position of the window. This number specified following seq number of the packet that the receiver expects to receive.
    pub window_position: u16,
    /// Position of written content. This number of a bit behind current window position and is increased every time packet is written into the file.
    pub next_write_position: u16,
    /// Temporary storage of parts received from the sender.
    /// This variable is freed when corresponding part is written into the file.
    pub parts_received: BTreeMap<u16, Vec<u8>>,
    /// When was last time receiver get packet from the sender.
    pub last_receive_time: Instant,
    /// Whether this connection received all the data and is closed by the sender (successfully).
    is_closed: bool,
    /// File into which store the received content.
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

    /// Check whether this connection end successfully and is closed.
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// Mark the connection as closed and flush content of the temp file.
    pub fn close(&mut self) {
        self.is_closed = true;
        self.file.take();
    }

    /// Check whether the connection timeouted.
    pub fn timeouted(&self, timeout: u32) -> bool {
        let threshold_time = Instant::now() - Duration::from_millis(timeout as u64);
        return self.last_receive_time < threshold_time;
    }

    /// Check whether the `ack` number is within windows of this connection.
    pub fn is_within_window(&self, ack: u16, config: &Config) -> bool {
        self.static_properties.is_within_window(ack, self.window_position, Box::new(config))
    }

    /// Store `data` received from the sender in packet with sequential number `seq` into cache memory.
    pub fn store_data(&mut self, data: &Vec<u8>, seq: u16, config: &Config) {
        // register new data
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
        while self.parts_received.contains_key(&self.window_position) {
            let new_pos = Wrapping::<u16>(self.window_position) + Wrapping::<u16>(1);
            self.window_position = new_pos.0;
        }
        config.vlog(&format!(
            "Window moved to position {} for connection {}",
            self.window_position,
            self.static_properties.id
        ));
    }

    /// Write data from the cache memory into the file if present.
    pub fn save_into_file(&mut self, config: &Config) {
        // path to the file
        let path_str = config.filename(self.static_properties.id);
        let path = Path::new(&path_str);

        // while there are packets to write
        while self.next_write_position != self.window_position {
            // get the following one and remove it from the cache memory
            let buffer = self.parts_received.remove(&self.next_write_position).expect("Part to write is not within the map");
            // make sure the file is open
            self.file = Some(match self.file.take() {
                Some(f) => f,
                None => OpenOptions::new().write(true)
                                          .append(true)
                                          .create(true)
                                          .open(path).expect("Can't open file for write")
            });
            let file = self.file.as_mut().unwrap();
            // write the content
            let wrote = file.write(&buffer).expect("Can't write to the output file");
            config.vlog(&format!(
                "Connection {} wrote {}b into file for packet seq {}",
                self.static_properties.id,
                wrote,
                self.next_write_position
            ));
            // move to the following packet
            let new_write_pos = Wrapping(self.next_write_position) + Wrapping::<u16>(1);
            self.next_write_position = new_write_pos.0;
        }
    }

    /// Get acknowledge number that the receiver should respond with.
    pub fn get_acknowledge(&self) -> u16 {
        let ack = Wrapping(self.window_position) - Wrapping::<u16>(1);
        return ack.0;
    }
}