use std::net::{SocketAddrV4, SocketAddr};
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};
use time::OffsetDateTime;
use crate::DATE_FORMAT_STR;

pub struct Config {
    pub verbose: bool,
    pub bind_addr: String,
    pub file: String,
    pub packet_size: u16,
    pub send_addr: String,
    pub window_size: u16,
    pub timeout: u32,
    pub repetition: u16,
    pub sum_size: u16,
}

impl Config {
    pub fn new() -> Self {
        return Config {
            verbose: false,
            bind_addr: String::from("127.0.0.1:3000"),
            file: String::from("input.txt"),
            packet_size: 1500,
            send_addr: String::from("127.0.0.1:3001"),
            window_size: 15,
            timeout: 100,
            repetition: 20,
            sum_size: 64,
        };
    }

    pub fn bind_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.bind_addr.as_str()).expect("Bind address is invalid");
    }

    pub fn send_addr(&self) -> SocketAddr {
        return SocketAddr::from_str(self.send_addr.as_str()).expect("Send address is invalid");
    }

    pub fn filename(&self) -> &str {
        return &self.file;
    }
    pub fn max_packet_size(&self) -> u16 {
        return self.packet_size;
    }
    pub fn is_verbose(&self) -> bool {
        return self.verbose;
    }
    pub fn timeout(&self) -> u32 {
        return self.timeout;
    }
    pub fn window_size(&self) -> u16 {
        return self.window_size;
    }
    pub fn repetitions(&self) -> u16 {
        return self.repetition;
    }
    pub fn checksum_size(&self) -> u16 {
        return self.sum_size;
    }

    pub fn vlog(&self, text: &str) {
        if self.verbose {
            println!("{}: {}", OffsetDateTime::now_utc().format(DATE_FORMAT_STR), text);
        }
    }

    pub fn from_command_line() -> Self {
        let mut config = Config::new();
        {
            let mut parser = ArgumentParser::new();
            parser.refer(&mut config.verbose)
                .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
            parser.refer(&mut config.bind_addr)
                .add_option(&["--bind"], Store, "Address to bind to in format IP:port");
            parser.refer(&mut config.file)
                .add_option(&["-f", "--file"], Store, "File to send")
                .required();
            parser.refer(&mut config.packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.refer(&mut config.send_addr)
                .add_option(&["--addr"], Store, "Address to send data in format IP:port");
            parser.refer(&mut config.window_size)
                .add_option(&["-w", "--window"], Store, "Size of the window");
            parser.refer(&mut config.timeout)
                .add_option(&["-t", "--timeout"], Store, "Timeout after starts to resend the data");
            parser.refer(&mut config.repetition)
                .add_option(&["-r", "--repetition"], Store, "How many times to resend packet");
            parser.refer(&mut config.sum_size)
                .add_option(&["-s", "--sum_size"], Store, "Size of the checksum");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

