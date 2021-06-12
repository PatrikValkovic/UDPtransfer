use std::{thread, thread::JoinHandle};
use std::cmp::min;
use std::collections::BinaryHeap;
use std::net::{SocketAddrV4, UdpSocket};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use rand::{distributions::Uniform, Rng, thread_rng};
use super::config::Config;
use super::packet_wrapper::PacketWrapper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::ErrorKind;

/// Creates the broker.
/// `brk` parameter should be set to `true` when the broker should terminate.
/// Returns handler to join the thread.
pub fn breakable_logic(config: Config, brk: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::Builder::new()
        .name(String::from("Broker"))
        .spawn(move || {
            broker(config, brk);
        }).expect("Can't create thread for the broker")
}

/// Creates the broker and keep running.
/// There is no way how to terminate the execution.
pub fn logic(config: Config) -> () {
    let brk = Arc::new(AtomicBool::new(false));
    broker(config, brk);
}

/// Creates the broker and spawn all the threads.
fn broker(config: Config, brk: Arc<AtomicBool>) -> () {
    // create sockets
    let send_socket = Arc::new(UdpSocket::bind(config.sender_bind()).expect("Can't bind sender socket"));
    let recv_socket = Arc::new(UdpSocket::bind(config.receiver_bind()).expect("Can't bind sender socket"));
    config.vlog(&format!("Sockets created --> {} <--> {} --> {}", config.sender_bind(), config.receiver_bind(), config.receiver_addr()));

    // create sender part
    let from_sender = handle(
        Arc::clone(&send_socket),
        Arc::clone(&recv_socket),
        config.clone(),
        config.receiver_addr(),
        "BrokerFromSender",
        brk.clone(),
    );
    // create receiver part
    let from_receiver = handle(
        Arc::clone(&recv_socket),
        Arc::clone(&send_socket),
        config.clone(),
        config.sender_addr(),
        "BrokerFromReceiver",
        brk.clone(),
    );

    // wait for them to end
    from_sender.join().expect("Can't join thread from sender");
    from_receiver.join().expect("Can't join threads from receiver");
}

/// Handles one part of the communication.
/// It receive packets from socket `send_socket` and resend them to `send_addr` from the `send_socket`.
fn handle(
    receive_socket: Arc<UdpSocket>,
    send_socket: Arc<UdpSocket>,
    config: Config,
    send_addr: SocketAddrV4,
    thread_name: &str,
    brk: Arc<AtomicBool>,
) -> JoinHandle<()> {
    let thread_name_copied = String::from(thread_name);
    thread::Builder::new().name(String::from(thread_name)).spawn(move || {
        let queue = Arc::new(Mutex::new(BinaryHeap::<PacketWrapper>::new()));
        let condvar = Arc::new(Condvar::new());

        let sending = sending_part(&config, &queue, &condvar, &send_socket, send_addr,
                                   &thread_name_copied, brk.clone());
        let receiving = receiving_part(&config, &queue, &condvar, &receive_socket,
                                       &thread_name_copied, brk.clone());

        sending.join().expect(&format!("Can't join sending part for the {}", thread_name_copied));
        receiving.join().expect(&format!("Can't join receiving part for the {}", thread_name_copied));
    }).expect(&format!("Can't create thread for {}", thread_name))
}

/// Handles receiving part of the communication.
/// It receives packets from `socket` and add them to the `queue`.
/// After adding content to the `queue` it notifies other thread (one) using `condvar` variable.
/// It decides about the delay, modification, and whether the packet should be dropped.
fn receiving_part(
    config: &Config,
    queue: &Arc<Mutex<BinaryHeap<PacketWrapper>>>,
    condvar: &Arc<Condvar>,
    socket: &Arc<UdpSocket>,
    thread_name: &str,
    brk: Arc<AtomicBool>,
) -> JoinHandle<()> {
    let config = config.clone();
    let queue = queue.clone();
    let condvar = condvar.clone();
    let socket = socket.clone();

    thread::Builder::new()
        .name(format!("{}_receive", thread_name))
        .spawn(move || {
            // create variables
            let mut buff = vec![0; 65535];
            let mut rand_gen = thread_rng();
            let probability_dist = Uniform::new(0.0, 1.0);
            let byte_dist = Uniform::new(0, 255);

            while !brk.load(Ordering::SeqCst) {
                // set socket timeout
                socket.set_read_timeout(Some(Duration::from_millis(1000)))
                      .expect("Can't change read timeout of the packet");
                // receive packet
                let (size, sender) = match socket.recv_from(buff.as_mut_slice()) {
                    Ok(x) => x,
                    Err(e) => {
                        let kind = e.kind();
                        if kind == ErrorKind::WouldBlock || kind == ErrorKind::TimedOut {
                            continue;
                        }
                        config.vlog(&format!("Could not receive from socket {:?}, ignoring", socket.local_addr()));
                        config.vlog(&format!("Error: {}", e.to_string()));
                        continue;
                    }
                };
                config.vlog(&format!("Received {}b of data from {}.", size, sender));

                // drop packet if dropout
                if rand_gen.sample(probability_dist) < config.droprate() {
                    config.vlog("Packet drop");
                    continue;
                }

                // modify packet and shorten it if necessary
                let content_length = min(size, config.max_packet_size() as usize);
                if config.modify_prob() > 0.0 {
                    for i in 0..content_length {
                        if rand_gen.sample(probability_dist) < config.modify_prob() {
                            buff[i] = rand_gen.sample(byte_dist);
                        }
                    }
                }
                let content = Vec::from(&buff[..content_length]);

                // get delay and create wrapper
                let delay: f32 = f32::max(0.0, config.delay_std() * rand_gen.gen::<f32>() + config.delay_mean());
                let wrapper = PacketWrapper::new(content, delay as u32);

                // add packet to the queue
                {
                    let mut queue = queue.lock().expect("Can't lock mutex from receiving part");
                    queue.push(wrapper);
                    condvar.notify_one();
                }
                config.vlog(&format!("Packet add to the queue"));
            }
        }).expect(&format!("Can't create receiving part of the {}", thread_name))
}

/// Handles sending part of the communication.
/// It pulls packets from the `queue` (after the required amount of time passed) and
/// send them to `sendaddr` using `socket`.
/// When new packet arrive into the `queue` it should be signaled using `condvar`.
fn sending_part(
    config: &Config,
    queue: &Arc<Mutex<BinaryHeap<PacketWrapper>>>,
    condvar: &Arc<Condvar>,
    socket: &Arc<UdpSocket>,
    send_addr: SocketAddrV4,
    thread_name: &str,
    brk: Arc<AtomicBool>,
) -> JoinHandle<()> {
    let config = config.clone();
    let queue = queue.clone();
    let condvar = condvar.clone();
    let socket = socket.clone();
    let tn = String::from(thread_name);

    thread::Builder::new()
        .name(String::from(format!("{}_send", thread_name)))
        .spawn(move || {
            while !brk.load(Ordering::SeqCst) {
                // get packet to send
                let to_send = {
                    // lock queue to get data
                    let mut queue_guard = queue.lock().expect("Can't lock mutex from the sender part");
                    // loop waiting for the packet to be send
                    while !brk.load(Ordering::SeqCst) {
                        // get the wait time based on the first packet that should be send
                        let wait_time = queue_guard.peek().map_or(Duration::from_secs(u64::MAX), |wrapper| { wrapper.send_in() });
                        // if it should be already send break the loop waiting for the packet
                        if wait_time.as_millis() == 0 {
                            break;
                        }
                        // wait time cannot be longer than 1s because of the termination
                        let wait_time = Duration::min(wait_time, Duration::from_secs(1));
                        // else wait specified time or until new packet (possibly with earlier sending time) is inserted
                        let result = condvar.wait_timeout(
                            queue_guard,
                            wait_time,
                        ).expect("Can't lock mutex from the sender part");
                        queue_guard = result.0;
                    };
                    // packet in the queue, pop it
                    let packet = match queue_guard.pop() {
                        Some(x) => x,
                        None => continue,
                    };
                    // validate once more it should be send already
                    if !packet.should_be_send() {
                        continue;
                    };
                    // return the packet from the loop
                    packet
                };

                // send packet
                match socket.send_to(to_send.content(), send_addr) {
                    Ok(send_size) => config.vlog(&format!("Send data of size {}b to {}", send_size, send_addr)),
                    Err(e) => eprintln!("Error sending data {}", e),
                };
            };
        }).expect(&format!("Can't create sender part of the {}", thread_name))
}


