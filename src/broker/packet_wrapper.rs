use std::time::{Instant, Duration};
use std::ops::Add;
use std::cmp::{Ord, Ordering};

/// Structure that stores data temporally before they are send.
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
        self.send_at
            .checked_duration_since(Instant::now())
            .unwrap_or_else(|| { Duration::from_secs(0) })
    }

    pub fn should_be_send(&self) -> bool {
        self.send_at < Instant::now()
    }

    pub fn content(&self) -> &Vec<u8> {
        &self.content
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