use std::net::{SocketAddrV4, SocketAddr};
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};
use crate::loggable::Loggable;

pub struct Config {
    pub verbose: bool,
    pub bind_addr: String,
    pub file: String,
    pub packet_size: u16,
    pub send_addr: String,
    pub window_size: u16,
    pub timeout: u32,
    pub repetition: u16,
    pub checksum_size: u16,
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
            checksum_size: 64,
        };
    }

    pub fn bind_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.bind_addr.as_str()).expect("Bind address is invalid");
    }
    pub fn send_addr(&self) -> SocketAddr {
        return SocketAddr::from_str(self.send_addr.as_str()).expect("Send address is invalid");
    }

    pub fn vlog(&self, text: &str) {
        Loggable::vlog(self, &text)
    }
    pub fn is_verbose(&self) -> bool {
        Loggable::is_verbose(self)
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
                .add_option(&["--addr"], Store, "Address where send data in format IP:port");
            parser.refer(&mut config.window_size)
                .add_option(&["-w", "--window"], Store, "Size of the window");
            parser.refer(&mut config.timeout)
                .add_option(&["-t", "--timeout"], Store, "Timeout after which resend the data");
            parser.refer(&mut config.repetition)
                .add_option(&["-r", "--repetition"], Store, "Maximum number of timeouts per packet");
            parser.refer(&mut config.checksum_size)
                .add_option(&["-s", "--sum_size"], Store, "Size of the checksum");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

impl Loggable for Config {
    fn is_verbose(&self) -> bool {
        self.verbose
    }
}
