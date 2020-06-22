use std::net::SocketAddrV4;
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};

pub struct Config {
    verbose: bool,
    bindaddr: String,
    sendaddr: String,
    packet_size: u32,
    delay_mean: f32,
    delay_std: f32,
    drop_rate: f32,
}

impl Config {
    pub fn new() -> Self {
        return Config {
            verbose: false,
            bindaddr: String::from("127.0.0.1:3001"),
            sendaddr: String::from("127.0.0.1:3002"),
            packet_size: 1500,
            delay_mean: 0.0,
            delay_std: 0.0,
            drop_rate: 0.0,
        };
    }

    pub fn bind_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.bindaddr.as_str()).expect("Invalid bind address");
    }

    pub fn send_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.sendaddr.as_str()).expect("Invalid send address");
    }

    pub fn max_packet_size(&self) -> u32 {
        return self.packet_size;
    }

    pub fn is_verbose(&self) -> bool {
        return self.verbose;
    }

    pub fn droprate(&self) -> f32 {
        return self.drop_rate;
    }

    pub fn from_command_line() -> Self {
        let mut config = Config::new();
        {
            let mut parser = ArgumentParser::new();
            parser.refer(&mut config.verbose)
                .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
            parser.refer(&mut config.bindaddr)
                .add_option(&["--addr"], Store, "Address to bind to in format ip:port");
            parser.refer(&mut config.sendaddr)
                .add_option(&["--addr"], Store, "Address to resend the data to in format ip:port");
            parser.refer(&mut config.packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.refer(&mut config.delay_mean)
                .add_option(&["-m", "--delay_mean"], Store, "Mean value of delay");
            parser.refer(&mut config.delay_std)
                .add_option(&["-s", "--delay_std"], Store, "Standard deviation of delay");
            parser.refer(&mut config.drop_rate)
                .add_option(&["-d", "--drop_rate"], Store, "Percentage of packets to drop between 0 and 1");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

