mod config;

use config::Config;
use std::net::{UdpSocket};
use rand::{thread_rng, Rng};
use rand::distributions::Uniform;

fn main() -> std::io::Result<()> {
    let config = Config::from_command_line();

    let socket = UdpSocket::bind(config.bind_addr()).expect("Can't bind socket");
    if config.is_verbose() {
        println!("Socket bind to {}", config.bind_addr());
    }

    let mut buff = Vec::new();
    buff.resize(config.max_packet_size() as usize, 0);
    let sendaddr = config.send_addr();
    let mut rand_gen = thread_rng();
    let unif = Uniform::new(0.0, 1.0);

    loop {
        let (size, sender) = socket.recv_from(buff.as_mut_slice()).expect("Can't receive data");
        if config.is_verbose() {
            println!("Received {}b of data from {}.", size, sender);
        }

        let dropval: f32 = rand_gen.sample(unif);
        if dropval > config.droprate() {
            let wrote = socket
                .send_to(&buff.as_slice()[..size], sendaddr)
                .expect("Couldn't send data");

            if config.is_verbose() {
                println!("Wrote {}b of data", wrote);
            }
        } else if config.is_verbose() {
            println!("Drop packet");
        }
    }
}