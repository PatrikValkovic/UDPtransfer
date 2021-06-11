use std::{thread, thread::JoinHandle};
use std::cmp::min;
use std::collections::BinaryHeap;
use std::net::{SocketAddrV4, UdpSocket};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use rand::{distributions::Uniform, Rng, thread_rng};

use super::config::Config;
use super::packet_wrapper::PacketWrapper;

pub fn broker(config: Config) -> () {
    let send_socket = Arc::new(UdpSocket::bind(config.sender_bind()).expect("Can't bind sender socket"));
    let recv_socket = Arc::new(UdpSocket::bind(config.receiver_bind()).expect("Can't bind sender socket"));
    config.vlog(&format!("Sockets created --> {} <--> {} --> {}", config.sender_bind(), config.receiver_bind(), config.receiver_addr()));

    let from_sender = handle(
        Arc::clone(&send_socket),
        Arc::clone(&recv_socket),
        config.clone(),
        config.receiver_addr(),
        "BrokerFromSender"
    );
    let from_receiver = handle(
        Arc::clone(&recv_socket),
        Arc::clone(&send_socket),
        config.clone(),
        config.sender_addr(),
        "BrokerFromReceiver"
    );

    from_sender.join().expect("Can't join sender part");
    from_receiver.join().expect("Can't join receiving part");
}


fn handle(
    receive_socket: Arc<UdpSocket>,
    send_socket: Arc<UdpSocket>,
    config: Config,
    send_addr: SocketAddrV4,
    threadname: &str,
) -> JoinHandle<()> {
    let name = String::from(threadname);
    return thread::Builder::new().name(String::from(threadname)).spawn(move || {
        let queue = Arc::new(Mutex::new(BinaryHeap::<PacketWrapper>::new()));
        let condvar = Arc::new(Condvar::new());

        sending_part(&config, &queue, &condvar, &send_socket, send_addr, &name);
        receiving_part(&config, &queue, &condvar, &receive_socket);
    }).expect(&format!("Can't create {} thread", threadname));
}

fn receiving_part(
    config: &Config,
    queue: &Arc<Mutex<BinaryHeap<PacketWrapper>>>,
    condvar: &Arc<Condvar>,
    socket: &Arc<UdpSocket>,
) {
    let mut buff = vec![0; 65535];
    let mut rand_gen = thread_rng();
    let unif = Uniform::new(0.0, 1.0);
    let byte_dist = Uniform::new(0,255);

    loop {
        let (size, sender) = match socket.recv_from(buff.as_mut_slice()) {
            Ok(x) => x,
            Err(e) => {
                println!("Could not receive from socket {:?}, ignoring", socket.local_addr());
                println!("{:?}", e);
                continue;
            }
        };
        config.vlog(&format!("Received {}b of data from {}.", size, sender));

        if rand_gen.sample(unif) > config.droprate() {
            let delay: f32 = f32::max(0.0, config.delay_std() * rand_gen.gen::<f32>() + config.delay_mean());
            let content_length = min(size, config.max_packet_size() as usize);
            for i in 0..content_length {
                if rand_gen.sample(unif) < config.modify_prob() {
                    buff[i] = rand_gen.sample(byte_dist);
                }
            }
            let content = Vec::from(&buff[..content_length]);


            let wrapper = PacketWrapper::new(content, delay as u32);

            {
                let mut queue = queue.lock().expect("Can't lock mutex from receiving part");
                queue.push(wrapper);
                condvar.notify_one();
            }
            config.vlog(&format!("Packet add to the queue"));
        } else {
            config.vlog(&format!("Drop packet"));
        }
    }
}

fn sending_part(
    config: &Config,
    queue: &Arc<Mutex<BinaryHeap<PacketWrapper>>>,
    condvar: &Arc<Condvar>,
    socket: &Arc<UdpSocket>,
    sendaddr: SocketAddrV4,
    threadname: &str
) -> JoinHandle<()> {
    let config = config.clone();
    let queue = Arc::clone(queue);
    let condvar = Arc::clone(condvar);
    let socket = Arc::clone(socket);

    thread::Builder::new().name(String::from(format!("{}_{}", threadname, "send")))
        .spawn(move || {
        loop {
            let to_send;
            {
                let packet = {
                    let mut queue_guard = queue.lock().expect("Can't lock mutex from the sender part");
                    loop {
                        let wait_time = queue_guard.peek().map_or(Duration::from_millis(u64::MAX), |wrapper| { wrapper.send_in() });
                        if wait_time.as_millis() == 0 {
                            break;
                        }
                        let result = condvar.wait_timeout(
                            queue_guard,
                            wait_time,
                        ).expect("Can't lock mutex from the sender part");
                        queue_guard = result.0;
                    };
                    queue_guard.pop()
                };
                to_send = match packet {
                    Some(x) => x,
                    None => {
                        config.vlog(&format!("No item in queue"));
                        continue;
                    }
                };
            };

            match socket.send_to(to_send.content(), sendaddr) {
                Ok(send_size) => config.vlog(&format!("Send data of size {}b to {}", send_size, sendaddr)),
                Err(e) => eprintln!("Error sending data {}", e),
            };
        };
    }).expect(&format!("Can't create sender part for {}", threadname))
}

