use std::net::{UdpSocket};
use std::fs::File;
use std::result::Result::Ok;
use std::io::Write;

use udp_transfer::receiver::config::Config;

fn main() -> std::io::Result<()> {
    let config = Config::from_command_line();

    let mut output_file = File::create(config.filename()).expect("Couldn't open file");
    if config.is_verbose() {
        println!("File {} opened", config.filename());
    }

    let socket = UdpSocket::bind(config.binding()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.binding());
    }

    let mut buff = Vec::new();
    buff.resize(config.max_packet_size() as usize, 0);

    while let Ok((size, sender)) = socket.recv_from(buff.as_mut_slice()){
        if config.is_verbose() {
            println!("Received {}b of data from {}.", size, sender);
        }

        let wrote = output_file.write(&buff.as_slice()[..size])?;
        if config.is_verbose(){
            println!("Wrote {}b of data", wrote);
        }
    };

    return Ok(());
}