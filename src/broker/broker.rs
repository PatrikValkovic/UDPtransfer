use crate::config::Config;
use crate::packet_wrapper::PacketWrapper;

use std::net::{UdpSocket, SocketAddrV4};
use std::thread::{JoinHandle, spawn};
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex, Condvar};
use std::{f32, u64};
use std::time::Duration;
use rand::{thread_rng, Rng, distributions::Uniform};

pub fn broker(config: Config) -> () {
    let send_socket = Arc::new(UdpSocket::bind(config.sender_bind()).expect("Can't bind sender socket"));
    let recv_socket = Arc::new(UdpSocket::bind(config.receiver_bind()).expect("Can't bind sender socket"));
    if config.is_verbose() {
        println!("Sockets created");
    };

    let from_sender = handle(
        Arc::clone(&send_socket),
        Arc::clone(&recv_socket),
        config.clone(),
        config.receiver_addr()
    );
    let from_receiver = handle(
        Arc::clone(&recv_socket),
        Arc::clone(&send_socket),
        config.clone(),
        config.sender_addr()
    );

    from_sender.join().expect("Can't join sender part");
    from_receiver.join().expect("Can't join receiving part");
}


fn handle(
    receive_socket: Arc<UdpSocket>,
    send_socket: Arc<UdpSocket>,
    config: Config,
    send_addr: SocketAddrV4,
) -> JoinHandle<()> {
    return spawn(move || {
        let queue = Arc::new(Mutex::new(BinaryHeap::<PacketWrapper>::new()));
        let condvar = Arc::new(Condvar::new());

        sending_part(&config, &queue, &condvar, &send_socket, send_addr);
        receiving_part(&config, &queue, &condvar, &receive_socket);
    });
}

fn receiving_part(
    config: &Config,
    queue: &Arc<Mutex<BinaryHeap<PacketWrapper>>>,
    condvar: &Arc<Condvar>,
    socket: &Arc<UdpSocket>,
) {
    let mut buff = Vec::new();
    buff.resize(config.max_packet_size() as usize, 0);
    let mut rand_gen = thread_rng();
    let unif = Uniform::new(0.0, 1.0);

    loop {
        let (size, sender) = socket.recv_from(buff.as_mut_slice()).expect("Can't receive data");
        if config.is_verbose() {
            println!("Received {}b of data from {}.", size, sender);
        }

        if rand_gen.sample(unif) > config.droprate() {
            let delay: f32 = f32::max(0.0, config.delay_std() * rand_gen.gen::<f32>() + config.delay_mean());
            let mut content = buff.clone();
            content.resize(size, 0);
            let wrapper = PacketWrapper::new(content, delay as u32);

            {
                let mut queue = queue.lock().expect("Can't lock mutex from receiving part");
                queue.push(wrapper);
                condvar.notify_one();
            }
            if config.is_verbose() {
                println!("Packet add to the queue");
            }
        } else if config.is_verbose() {
            println!("Drop packet");
        }
    }
}

fn sending_part(
    config: &Config,
    queue: &Arc<Mutex<BinaryHeap<PacketWrapper>>>,
    condvar: &Arc<Condvar>,
    socket: &Arc<UdpSocket>,
    sendaddr: SocketAddrV4,
) -> JoinHandle<()> {
    let config = config.clone();
    let queue = Arc::clone(queue);
    let condvar = Arc::clone(condvar);
    let socket = Arc::clone(socket);

    spawn(move || {
        loop {
            let to_send;
            {
                let mut queue = loop {
                    let queue_guard = queue.lock().expect("Can't lock mutex from the sender part");
                    let wait_time = queue_guard.peek().map_or(Duration::from_millis(u64::MAX), |wrapper| { wrapper.send_in() });
                    if wait_time.as_millis() == 0 {
                        break queue_guard;
                    }
                    condvar.wait_timeout(
                        queue_guard,
                        wait_time,
                    ).expect("Can't lock mutex from the sender part");
                };
                to_send = match queue.pop() {
                    Some(x) => x,
                    None if config.is_verbose() => {
                        println!("No item in queue");
                        continue;
                    }
                    None => continue
                };
            };

            match socket.send_to(to_send.content(), sendaddr) {
                Ok(send_size) if config.is_verbose() => println!("Send data of size {}b", send_size),
                Ok(_) => {}
                Err(e) => eprintln!("Error sending data {}", e),
            };
        };
    })
}

