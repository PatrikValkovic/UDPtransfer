use std::net::SocketAddrV4;
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};
use std::path::PathBuf;
use crate::loggable::Loggable;

pub struct Config {
    pub verbose: bool,
    pub bindaddr: String,
    pub directory: String,
    pub max_packet_size: u16,
    pub max_window_size: u16,
    pub min_checksum: u16,
    pub timeout: u32,
}

impl Config {
    pub fn new() -> Self {
        return Config {
            verbose: false,
            bindaddr: String::from("127.0.0.1:3003"),
            directory: String::from("received"),
            max_packet_size: 1500,
            max_window_size: 15,
            min_checksum: 16,
            timeout: 5000,
        };
    }

    pub fn binding(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.bindaddr.as_str()).expect("Invalid bind address");
    }

    pub fn filename(&self, connection_id: u32) -> String {
        let mut path = PathBuf::new();
        path.push(&self.directory);
        path.push(connection_id.to_string());
        let final_path = String::from(path.as_path().to_str().unwrap());
        return final_path;
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
            parser.refer(&mut config.bindaddr)
                .add_option(&["--addr"], Store, "Address to bind to in format IP:port");
            parser.refer(&mut config.directory)
                .add_option(&["-d", "--directory"], Store, "Directory where to store received files");
            parser.refer(&mut config.max_packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.refer(&mut config.max_window_size)
                .add_option(&["-w", "--window"], Store, "Maximum size of the window");
            parser.refer(&mut config.timeout)
                .add_option(&["-t", "--timeout"], Store, "Timeout after which resend the acknowledge packet");
            parser.refer(&mut config.min_checksum)
                .add_option(&["-s", "--checksum"], Store, "Minimum size of checksum");
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
