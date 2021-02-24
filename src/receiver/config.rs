use std::net::SocketAddrV4;
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};
use std::ops::Add;

pub struct Config {
    verbose: bool,
    bindaddr: String,
    directory: String,
    max_packet_size: u16,
    max_window_size: u16,
    min_checksum: u16,
    timeout: u32,
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
        return String::from(&self.directory).add("/").add(&connection_id.to_string());
    }

    pub fn max_packet_size(&self) -> u16 {
        return self.max_packet_size;
    }
    pub fn max_window_size(&self) -> u16 {
        return self.max_window_size;
    }
    pub fn min_checksum_size(&self) -> u16 {
        return self.min_checksum;
    }
    pub fn get_timeout(&self) -> u32 {
        return self.timeout;
    }
    pub fn is_verbose(&self) -> bool {
        return self.verbose;
    }

    pub fn vlog(&self, text: &str){
        if self.is_verbose() {
            println!("{}", text);
        }
    }

    pub fn from_command_line() -> Self {
        let mut config = Config::new();
        {
            let mut parser = ArgumentParser::new();
            parser.refer(&mut config.verbose)
                .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
            parser.refer(&mut config.bindaddr)
                .add_option(&["--addr"], Store, "Address to bind to in format ip:port");
            parser.refer(&mut config.directory)
                .add_option(&["-d", "--directory"], Store, "Directory where to store received files");
            parser.refer(&mut config.max_packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.refer(&mut config.max_window_size)
                .add_option(&["-w", "--window"], Store, "Size of the window");
            parser.refer(&mut config.timeout)
                .add_option(&["-t", "--timeout"], Store, "Timeout after starts to resend the data");
            parser.refer(&mut config.min_checksum)
                .add_option(&["-s", "--checksum"], Store, "Minimum size of checksum");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

