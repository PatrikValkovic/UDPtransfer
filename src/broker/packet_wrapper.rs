use std::time::{Instant, Duration};
use std::ops::Add;
use std::cmp::{Ord, Ordering};

pub struct PacketWrapper {
    content: Vec<u8>,
    send_at: Instant,
}

impl PacketWrapper {
    pub fn new(content: Vec<u8>, send_in_millis: u32) -> PacketWrapper {
        let send_at = Instant::now().add(Duration::from_millis(send_in_millis as u64));
        return PacketWrapper {
            content,
            send_at,
        };
    }

    pub fn send_in(&self) -> Duration {
        if Instant::now() > self.send_at {
            return Instant::now() - self.send_at;
        } else {
            return Duration::from_millis(0);
        }
    }

    pub fn content(&self) -> &Vec<u8> {
        return &self.content;
    }
}

impl Ord for PacketWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        return self.send_at.cmp(&other.send_at);
    }
}

impl PartialOrd for PacketWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return self.send_at.partial_cmp(&other.send_at);
    }
}

impl PartialEq for PacketWrapper {
    fn eq(&self, other: &Self) -> bool {
        return self.send_at.eq(&other.send_at);
    }
}

impl Eq for PacketWrapper {}