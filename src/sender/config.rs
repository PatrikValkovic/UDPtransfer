use std::net::SocketAddrV4;
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};

pub struct Config {
    verbose: bool,
    bindaddr: String,
    file: String,
    packet_size: u32,
    sendaddr: String,
}

impl Config {
    pub fn new() -> Self {
        return Config {
            verbose: false,
            bindaddr: String::from("127.0.0.1:3000"),
            file: String::from("input.txt"),
            packet_size: 1500,
            sendaddr: String::from("127.0.0.1:3001"),
        };
    }

    pub fn bind_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.bindaddr.as_str()).expect("Bind address is invalid");
    }

    pub fn send_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.sendaddr.as_str()).expect("Send address is invalid");
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
                .add_option(&["--bind"], Store, "Address to bind to in format IP:port");
            parser.refer(&mut config.file)
                .add_option(&["-f", "--file"], Store, "File to send")
                .required();
            parser.refer(&mut config.packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.refer(&mut config.sendaddr)
                .add_option(&["--send"], Store, "Address to send data in format IP:port");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

