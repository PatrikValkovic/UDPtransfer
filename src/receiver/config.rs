use std::net::{Ipv4Addr, SocketAddrV4};
use argparse::{ArgumentParser, StoreTrue, Store};

pub struct Config {
    verbose: bool,
    ip_addr: Ipv4Addr,
    port: u32,
    file: String,
    packet_size: u32,
}

impl Config {
    pub fn new() -> Self {
        return Config {
            verbose: false,
            ip_addr: Ipv4Addr::new(127, 0, 0, 1),
            port: 3000,
            file: String::from("output.txt"),
            packet_size: 1500,
        };
    }

    pub fn binding(&self) -> SocketAddrV4 {
        return SocketAddrV4::new(self.ip_addr, self.port as u16);
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
            parser.refer(&mut config.ip_addr)
                .add_option(&["--ip"], Store, "Bind IP address");
            parser.refer(&mut config.port)
                .add_option(&["--port"], Store, "Bind port");
            parser.refer(&mut config.file)
                .add_option(&["-f", "--file"], Store, "File to store");
            parser.refer(&mut config.packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

