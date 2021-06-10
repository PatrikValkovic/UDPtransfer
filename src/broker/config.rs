use std::net::{SocketAddrV4};
use std::str::FromStr;
use argparse::{ArgumentParser, StoreTrue, Store};

#[derive(Clone)]
pub struct Config {
    pub verbose: bool,
    pub sender_bindaddr: String,
    pub sender_addr: String,
    pub receiver_bindaddr: String,
    pub receiver_addr: String,
    pub packet_size: u32,
    pub delay_mean: f32,
    pub delay_std: f32,
    pub drop_rate: f32,
    pub modify_prob: f32,
}

impl Config {
    pub fn new() -> Self {
        return Config {
            verbose: false,
            sender_bindaddr: String::from("127.0.0.1:3001"),
            sender_addr: String::from("127.0.0.1:3000"),
            receiver_bindaddr: String::from("127.0.0.1:3002"),
            receiver_addr: String::from("127.0.0.1:3003"),
            packet_size: 1500,
            delay_mean: 0.0,
            delay_std: 0.0,
            drop_rate: 0.0,
            modify_prob: 0.0,
        };
    }

    pub fn sender_bind(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.sender_bindaddr.as_str()).expect("Invalid bind address for the sender");
    }
    pub fn sender_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.sender_addr.as_str()).expect("Invalid address of the sender");
    }
    pub fn receiver_bind(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.receiver_bindaddr.as_str()).expect("Invalid bind address for the receiver");
    }
    pub fn receiver_addr(&self) -> SocketAddrV4 {
        return SocketAddrV4::from_str(self.receiver_addr.as_str()).expect("Invalid address of the receiver");
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
    pub fn delay_mean(&self) -> f32 {
        return self.delay_mean;
    }
    pub fn delay_std(&self) -> f32 {
        return self.delay_std;
    }
    pub fn modify_prob(&self) -> f32 {
        return self.modify_prob;
    }

    pub fn from_command_line() -> Self {
        let mut config = Config::new();
        {
            let mut parser = ArgumentParser::new();
            parser.refer(&mut config.verbose)
                .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
            parser.refer(&mut config.sender_bindaddr)
                .add_option(&["--sender_bind"], Store, "Address to bind from the sender perspective in format ip:port");
            parser.refer(&mut config.receiver_bindaddr)
                .add_option(&["--receiver_bind"], Store, "Address to bind from the receiver perspective in format ip:port");
            parser.refer(&mut config.sender_addr)
                .add_option(&["--sender_addr"], Store, "Address of the sender in format ip:port");
            parser.refer(&mut config.receiver_addr)
                .add_option(&["--receiver_addr"], Store, "Address of the receiver in format ip:port");
            parser.refer(&mut config.packet_size)
                .add_option(&["--packet"], Store, "Maximum packet size");
            parser.refer(&mut config.delay_mean)
                .add_option(&["-m", "--delay_mean"], Store, "Mean value of delay");
            parser.refer(&mut config.delay_std)
                .add_option(&["-s", "--delay_std"], Store, "Standard deviation of delay");
            parser.refer(&mut config.drop_rate)
                .add_option(&["-d", "--drop_rate"], Store, "Percentage of packets to drop between 0 and 1");
            parser.refer(&mut config.modify_prob)
                .add_option(&["-m", "--modify"], Store, "Probability of byte modification");
            parser.parse_args_or_exit();
        }
        return config;
    }
}

