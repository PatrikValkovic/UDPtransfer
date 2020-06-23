use std::vec::Vec;
use byteorder::{NetworkEndian, ByteOrder};


pub struct Packet {
    pub id: u32,
    pub seq: u16,
    pub ack: u16,
    pub flag: u8,
    pub data: Vec<u8>,
}

fn num_blocks(data: usize, checksum_size: usize) -> usize {
    return match (data / checksum_size, data % checksum_size) {
        (div, 0) => div,
        (div, modulo) => div + 1
    };
}
fn construct_checksum(data: &[u8], checksum_size: usize) -> Vec<u8> {
    let mut checksum = vec![0; checksum_size];
    let blocks = num_blocks(data.len(), checksum_size);

    for i in 0..blocks {
        let ending = usize::min((i + 1) * checksum_size, data.len());
        let block = &data[i * checksum_size..ending];
        checksum.iter_mut()
            .zip(block.iter())
            .for_each(|(orig, new)| {
                *orig = *orig ^ *new;
            });
    };

    return checksum;
}

fn check_checksum(first: &Vec<u8>, second: &Vec<u8>) -> bool {
    if let Some(e) = first.iter()
        .zip(second.iter())
        .find(|(&comp, &inside)| { comp != inside }) {
        return true;
    }
    else {
        return false;
    }
}


impl Packet {
    pub fn new(data: &[u8], checksum_size: usize) -> Result<Self, &str> {
        let len = data.len();
        if len < 9 + checksum_size {
            return Err("Not enough data received");
        }
        let data_start = 9;
        let data_end = len - checksum_size;

        let id = NetworkEndian::read_u32(&data[..4]);
        let seq = NetworkEndian::read_u16(&data[4..6]);
        let ack = NetworkEndian::read_u16(&data[6..8]);
        let flag = data[8];
        let data_vec = Vec::from(&data[data_start..data_end]);

        if checksum_size > 0 {
            let checksum_data = Vec::from(&data[len - checksum_size..len]);
            let checksum = construct_checksum(&data[..data_end], checksum_size);
            if check_checksum(&checksum_data, &checksum) {
                return Err("Checksum doesn't match");
            }
        }

        return Ok(Packet {
            id,
            seq,
            ack,
            flag,
            data: data_vec,
        });
    }

    pub fn to_binary(&self, checksum_size: usize, vect: &mut Vec<u8>) -> usize {
        let data_end = 9 + self.data.len();
        let buffer_size = data_end + checksum_size;
        vect.resize(buffer_size, 0);

        NetworkEndian::write_u32(&mut vect[..4], self.id);
        NetworkEndian::write_u16(&mut vect[4..6], self.seq);
        NetworkEndian::write_u16(&mut vect[6..8], self.ack);
        vect[8] = self.flag;
        vect[9..data_end].copy_from_slice(self.data.as_slice());

        if checksum_size > 0 {
            let checksum = construct_checksum(&vect[..data_end], checksum_size);
            vect[data_end..].copy_from_slice(checksum.as_slice());
        }

        return buffer_size;
    }
}

#[cfg(test)]
mod tests {

    mod new {
        use crate::packet::Packet;

        #[test]
        fn should_parse_successfully() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                7 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            let packet = Packet::new(&data.as_slice(), 4).unwrap();
            assert_eq!(packet.id, 1 << 8);
            assert_eq!(packet.seq, 5);
            assert_eq!(packet.ack, 8);
            assert_eq!(packet.flag, 7);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7]);
        }

        #[test]
        fn not_aligned_to_block() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                11, 13, 17, //data
                7 ^ 4 ^ 11, 5 ^ 1 ^ 5 ^ 13, 1 ^ 2 ^ 6 ^ 17, 8 ^ 3 ^ 7
            ];
            let packet = Packet::new(&data.as_slice(), 4).unwrap();
            assert_eq!(packet.id, 1 << 8);
            assert_eq!(packet.seq, 5);
            assert_eq!(packet.ack, 8);
            assert_eq!(packet.flag, 7);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17]);
        }

        #[test]
        fn checksum_not_match() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                7 ^ 4, 5 ^ 1 ^ 5, /*1 ^*/ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            let packet = Packet::new(&data.as_slice(), 4).err().unwrap();
            assert_eq!("Checksum doesn't match", packet);
        }

        #[test]
        fn data_not_match() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                /*1*/0, 2, 3, //data
                4, 5, 6, 7, //data
                7 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            let packet = Packet::new(&data.as_slice(), 4).err().unwrap();
            assert_eq!("Checksum doesn't match", packet);
        }

        #[test]
        fn data_too_short() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                // no data
                7 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6/*, 8 ^ 3 ^ 7*/
            ];
            let packet = Packet::new(&data.as_slice(), 4).err().unwrap();
            assert_eq!("Not enough data received", packet);
        }

        #[test]
        fn without_checksum() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, //data
            ];
            let packet = Packet::new(&data.as_slice(), 0).unwrap();
            assert_eq!(packet.id, 1 << 8);
            assert_eq!(packet.seq, 5);
            assert_eq!(packet.ack, 8);
            assert_eq!(packet.flag, 7);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6]);
        }
    }

    mod to_binary {
        use crate::packet::Packet;

        #[test]
        fn valid_transfer() {
            let packet = Packet {
                id: 1 << 8,
                seq: 5,
                ack: 8,
                flag: 7,
                data: vec![1, 2, 3, 4, 5, 6, 7]
            };
            let mut actual = Vec::new();
            let wrote = packet.to_binary(4, &mut actual);
            let expected: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                7 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            assert_eq!(wrote, expected.len());
            assert_eq!(actual, expected);
        }

        #[test]
        fn not_aligned_to_block() {
            let packet = Packet {
                id: 1 << 8,
                seq: 5,
                ack: 8,
                flag: 7,
                data: vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17]
            };
            let mut actual = Vec::new();
            let wrote = packet.to_binary(4, &mut actual);
            let expected: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                11, 13, 17, //data
                7 ^ 4 ^ 11, 5 ^ 1 ^ 5 ^ 13, 1 ^ 2 ^ 6 ^ 17, 8 ^ 3 ^ 7
            ];
            assert_eq!(wrote, expected.len());
            assert_eq!(actual, expected);
        }

        #[test]
        fn no_checksum() {
            let packet = Packet {
                id: 1 << 8,
                seq: 5,
                ack: 8,
                flag: 7,
                data: vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17]
            };
            let mut actual = Vec::new();
            let wrote = packet.to_binary(0, &mut actual);
            let expected: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                11, 13, 17, //data
            ];
            assert_eq!(wrote, expected.len());
            assert_eq!(actual, expected);
        }
    }
}
