use std::net::SocketAddrV4;
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};

pub struct Config {
    verbose: bool,
    bindaddr: String,
    file: String,
    packet_size: u32,
    window_size: u16,
    timeout: u32,
}

impl Config {
    pub fn new() -> Self {
        return Config {
            verbose: false,
            bindaddr: String::from("127.0.0.1:3003"),
            file: String::from("output.txt"),
            packet_size: 1500,
            window_size: 15,
            timeout: 2000,
        };
    }

    pub fn binding(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.bindaddr.as_str()).expect("Invalid bind address");
    }

    pub fn filename(&self) -> &str {
        return &self.file;
    }

    pub fn max_packet_size(&self) -> u32 {
        return self.packet_size;
    }

    pub fn is_verbose(&self) -> bool {
        return self.verbose;
    }

    pub fn from_command_line() -> Self {
        let mut config = Config::new();
        {
            let mut parser = ArgumentParser::new();
            parser.refer(&mut config.verbose)
                .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
            parser.refer(&mut config.bindaddr)
                .add_option(&["--addr"], Store, "Address to bind to in format ip:port");
            parser.refer(&mut config.file)
                .add_option(&["-f", "--file"], Store, "File to store");
            parser.refer(&mut config.packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.refer(&mut config.window_size)
                .add_option(&["-w", "--window"], Store, "Size of the window");
            parser.refer(&mut config.timeout)
                .add_option(&["-t", "--timeout"], Store, "Timeout after starts to resend the data");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

