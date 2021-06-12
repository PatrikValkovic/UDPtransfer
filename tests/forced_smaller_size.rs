use udp_transfer::{receiver, sender, broker};
use std::thread;
use std::fs::{File, read_dir, remove_file, remove_dir_all, create_dir_all};
use rand::{RngCore};
use std::io::{Write, Read};
use std::time::Duration;
use itertools::zip;

#[test]
fn forced_smaller_size(){
    const SOURCE_FILE: &str = "somefile.txt";
    const TARGET_DIR: &str = "received";
    const FILE_SIZE: usize = 2 * 1024 * 1024;
    const RECEIVED_ADDR: &str = "127.0.0.1:3100";
    const SENDER_ADDR: &str = "127.0.0.1:3101";
    const BROKER_RECV_PART: &str = "127.0.0.1:3102";
    const BROKER_SEND_PART: &str = "127.0.0.1:3103";

    // create 2MB file and directory
    {
        match remove_file(SOURCE_FILE) { _ => {}};
        match remove_dir_all(TARGET_DIR) { _ => {}};
        create_dir_all(TARGET_DIR).unwrap();
        let mut file = File::create(SOURCE_FILE).unwrap();
        let mut rng = rand::thread_rng();
        let mut buffer = vec![0; FILE_SIZE];
        rng.fill_bytes(&mut buffer);
        file.write_all(&buffer).unwrap();
    }

    // create receiver
    thread::Builder::new().name(String::from("Receiver")).spawn(|| {
        let rc = receiver::config::Config {
            verbose: false,
            bindaddr: String::from(RECEIVED_ADDR),
            directory: String::from(TARGET_DIR),
            max_packet_size: 1500,
            max_window_size: 15,
            min_checksum: 0,
            timeout: 5000
        };
        receiver::logic::logic(rc).unwrap();
    }).unwrap();

    // create broker
    thread::Builder::new().name(String::from("Broker")).spawn(|| {
        let bc = broker::config::Config {
            verbose: false,
            sender_bindaddr: String::from(BROKER_SEND_PART),
            sender_addr: String::from(SENDER_ADDR),
            receiver_bindaddr: String::from(BROKER_RECV_PART),
            receiver_addr: String::from(RECEIVED_ADDR),
            packet_size: 800,
            delay_mean: 0.0,
            delay_std: 0.0,
            drop_rate: 0.0,
            modify_prob: 0.0
        };
        broker::logic(bc);
    }).unwrap();

    // create sender
    let st = thread::Builder::new().name(String::from("Sender")).spawn(|| {
        let sc = sender::config::Config {
            verbose: false,
            bind_addr: String::from(SENDER_ADDR),
            file: String::from(SOURCE_FILE),
            packet_size: 1500,
            send_addr: String::from(BROKER_SEND_PART),
            window_size: 15,
            timeout: 100,
            repetition: 10,
            sum_size: 0
        };
        sender::logic::logic(sc).unwrap();
    }).unwrap();

    // wait for sender and kill receiver afterwards
    st.join().unwrap();
    thread::sleep(Duration::from_secs(1));

    // compare files
    {
        let mut original = File::open(SOURCE_FILE).unwrap();
        let mut orig_vector = vec![0; FILE_SIZE];
        assert_eq!(original.read(&mut orig_vector).unwrap(), FILE_SIZE);
        let mut directory_read = read_dir(TARGET_DIR).unwrap();
        let received_file = directory_read.next().unwrap().unwrap();
        let path_to_received_file = String::from(received_file.path().to_str().unwrap());
        let mut received = File::open(path_to_received_file).unwrap();
        let mut received_vector = vec![0; FILE_SIZE];
        assert_eq!(received.read(&mut received_vector).unwrap(), FILE_SIZE);
        for (o, r) in zip(&orig_vector, &received_vector) {
            assert_eq!(o, r);
        }
    }

    // delete files
    remove_file(SOURCE_FILE).unwrap();
    remove_dir_all(TARGET_DIR).unwrap();
}