use udp_transfer::{receiver, sender};
use std::fs::{File, read_dir, remove_file, remove_dir_all, create_dir_all};
use rand::{Rng};
use std::io::{Write, Read};
use itertools::zip;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn smaller_packet_size(){
    const SOURCE_FILE: &str = "somefile.txt";
    const TARGET_DIR: &str = "received";
    const FILE_SIZE: usize = 2 * 1024 * 1024;
    const RECEIVER_ADDR: &str = "127.0.0.1:3100";
    const SENDER_ADDR: &str = "127.0.0.1:3101";

    // create 2MB file and directory
    {
        match remove_file(SOURCE_FILE) { _ => {}};
        match remove_dir_all(TARGET_DIR) { _ => {}};
        create_dir_all(TARGET_DIR).unwrap();
        let mut file = File::create(SOURCE_FILE).unwrap();
        let mut rng = rand::thread_rng();
        let mut buffer = vec![0; FILE_SIZE];
        for f in buffer.as_mut_slice() {
            *f = rng.gen::<u8>();
        }
        file.write_all(&buffer).unwrap();
    }

    // create receiver
    let receiver_brk = Arc::new(AtomicBool::new(false));
    let rc = receiver::config::Config {
        verbose: false,
        bindaddr: String::from(RECEIVER_ADDR),
        directory: String::from(TARGET_DIR),
        max_packet_size: 800,
        max_window_size: 15,
        min_checksum: 0,
        timeout: 5000
    };
    let rt = receiver::breakable_logic(rc, receiver_brk.clone());

    // create sender
    let sender_brk = Arc::new(AtomicBool::new(false));
    let sc = sender::config::Config {
        verbose: false,
        bind_addr: String::from(SENDER_ADDR),
        file: String::from(SOURCE_FILE),
        packet_size: 1500,
        send_addr: String::from(RECEIVER_ADDR),
        window_size: 15,
        timeout: 100,
        repetition: 10,
        checksum_size: 0
    };
    let st= sender::breakable_logic(sc, sender_brk);

    // wait for sender
    st.join().unwrap().unwrap();

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

    // end receiver
    receiver_brk.store(true, Ordering::SeqCst);
    rt.join().unwrap().unwrap();

    // delete files
    remove_file(SOURCE_FILE).unwrap();
    remove_dir_all(TARGET_DIR).unwrap();
}