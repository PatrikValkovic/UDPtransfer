use std::vec::Vec;
use byteorder::{NetworkEndian, ByteOrder};
use crate::transferable::Transferable;

#[derive(Debug, PartialEq)]
pub enum Flag {
    None,
    Init,
    Data,
    Error,
    End,
}

impl Transferable for Flag {
    fn bin_size(&self) -> usize {
        return 1;
    }
    fn to_bin_buff(&self, buff: &mut [u8]) {
        buff[0] = match self {
            Flag::None => 0x0,
            Flag::Init => 0x1,
            Flag::Data => 0x2,
            Flag::Error => 0x4,
            Flag::End => 0x8,
        };
    }
    fn from_bin(val: &[u8]) -> Self {
        return match val[0] {
            0x1 => Flag::Init,
            0x2 => Flag::Data,
            0x4 => Flag::Error,
            0x8 => Flag::End,
            _ => Flag::None,
        };
    }
}


pub struct PacketHeader {
    pub id: u32,
    pub seq: u16,
    pub ack: u16,
    pub flag: Flag,
}

impl Transferable for PacketHeader {
    fn bin_size(&self) -> usize {
        return 9;
    }

    fn to_bin_buff(&self, buff: &mut [u8]) {
        NetworkEndian::write_u32(&mut buff[..4], self.id);
        NetworkEndian::write_u16(&mut buff[4..6], self.seq);
        NetworkEndian::write_u16(&mut buff[6..8], self.ack);
        self.flag.to_bin_buff(&mut buff[8..9]);
    }

    fn from_bin(memory: &[u8]) -> Self {
        let id = NetworkEndian::read_u32(&memory[..4]);
        let seq = NetworkEndian::read_u16(&memory[4..6]);
        let ack = NetworkEndian::read_u16(&memory[6..8]);
        let flag = Flag::from_bin(&memory[8..9]);
        return PacketHeader {
            id,
            seq,
            ack,
            flag,
        };
    }
}


pub struct InitPacket {
    pub header: PacketHeader,
    pub window_size: u16,
    pub packet_size: u16,
    pub checksum_size: u16,
}

impl Transferable for InitPacket {
    fn bin_size(&self) -> usize {
        debug_assert!(self.header.bin_size() + 6 < self.packet_size as usize);
        return self.packet_size as usize;
    }

    fn to_bin_buff(&self, buff: &mut [u8]) {
        debug_assert!(buff.len() == self.packet_size as usize);
        let header_size = self.header.bin_size() as usize;
        let checksum_size = self.checksum_size as usize;
        let packet_size = self.packet_size as usize;

        self.header.to_bin_buff(buff);
        NetworkEndian::write_u16(&mut buff[header_size..header_size + 2], self.window_size);
        NetworkEndian::write_u16(&mut buff[header_size + 2..header_size + 4], self.packet_size);
        NetworkEndian::write_u16(&mut buff[header_size + 4..header_size + 6], self.checksum_size);
        buff[header_size + 6..].copy_from_slice(vec![0; packet_size - header_size - 6].as_slice());
        let checksum = construct_checksum(&buff[..packet_size - checksum_size], checksum_size);
        buff[packet_size - checksum_size..].copy_from_slice(&checksum);
    }

    fn from_bin(memory: &[u8]) -> Self {
        let header = PacketHeader::from_bin(memory);
        let header_size = header.bin_size() as usize;
        let window_size = NetworkEndian::read_u16(&memory[header_size..header_size + 2]);
        let packet_size = NetworkEndian::read_u16(&memory[header_size + 2..header_size + 4]);
        let checksum_size = NetworkEndian::read_u16(&memory[header_size + 4..header_size + 6]);
        debug_assert!(memory.len() == packet_size as usize);

        let checksum = construct_checksum(&memory[..(packet_size - checksum_size) as usize], checksum_size as usize);
        let mut checksum_read = vec![0; checksum_size as usize];
        checksum_read.as_mut_slice().copy_from_slice(&memory[(packet_size - checksum_size) as usize..]);
        if !check_checksum(&checksum, &checksum_read) {
            panic!("Checksums do not match"); //TODO handle
        }

        return InitPacket {
            header,
            window_size,
            packet_size,
            checksum_size,
        };
    }
}

impl InitPacket {
    pub fn new(window_size: u16, packet_size: u16, checksum_size: u16) -> Self {
        return InitPacket {
            header: PacketHeader {
                id: 0,
                seq: 0,
                ack: 0,
                flag: Flag::Init,
            },
            window_size,
            packet_size,
            checksum_size
        };
    }
}


pub struct DataPacket {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

impl /*Transferable for*/ DataPacket {
    //don't know checksum size, so can't create full packet
    fn bin_size(&self) -> usize {
        return self.header.bin_size() + self.data.len();
    }

    fn to_bin_buff(&self, buff: &mut [u8]) {
        let header_size = self.header.bin_size();
        self.header.to_bin_buff(buff);
        buff[header_size..].copy_from_slice(self.data.as_slice());
    }

    fn from_bin(memory: &[u8]) -> Self {
        let header = PacketHeader::from_bin(memory);
        let header_size = header.bin_size();
        let data = Vec::from(&memory[header_size..]);

        return Self {
            header,
            data,
        };
    }
}

impl DataPacket {
    pub fn new(data: Vec<u8>, connection_id: u32, seq: u16, ack: u16) -> Self {
        return DataPacket {
            header: PacketHeader {
                id: connection_id,
                seq,
                ack,
                flag: Flag::Data
            },
            data,
        };
    }

    pub fn from(data: &[u8], checksum_size: usize) -> Result<Self, &str> {
        let len = data.len();
        if len < 9 + checksum_size {
            return Err("Not enough data received"); //TODO handle
        }
        let data_end = len - checksum_size;

        if checksum_size > 0 {
            let checksum_data = Vec::from(&data[len - checksum_size..len]);
            let checksum = construct_checksum(&data[..data_end], checksum_size);
            if check_checksum(&checksum_data, &checksum) {
                return Err("Checksum doesn't match"); //TODO handle
            }
        }
        return Ok(DataPacket::from_bin(&data[..data_end]));
    }

    pub fn to_bin_with_checksum(&self, checksum_size: usize, buff: &mut [u8]) -> usize {
        let header_size = self.header.bin_size();
        let packet_size = header_size + self.data.len() + checksum_size;
        let data_end = self.header.bin_size() + self.data.len();
        debug_assert!(buff.len() >= packet_size);

        self.to_bin_buff(&mut buff[..data_end]);

        if checksum_size > 0 {
            let checksum = construct_checksum(&buff[..data_end], checksum_size);
            buff[data_end..].copy_from_slice(checksum.as_slice());
        }

        return packet_size;
    }
}


fn num_blocks(data: usize, checksum_size: usize) -> usize {
    return match (data / checksum_size, data % checksum_size) {
        (div, 0) => div,
        (div, _modulo) => div + 1
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
    if let Some(_) = first.iter()
        .zip(second.iter())
        .find(|(&comp, &inside)| { comp != inside }) {
        return true;
    } else {
        return false;
    }
}


#[cfg(test)]
mod tests {
    mod new {
        use crate::packet::DataPacket;
        use crate::packet::Flag;

        #[test]
        fn should_parse_successfully() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                4 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            let packet = DataPacket::from(&data.as_slice(), 4).unwrap();
            assert_eq!(packet.header.id, 1 << 8);
            assert_eq!(packet.header.seq, 5);
            assert_eq!(packet.header.ack, 8);
            assert_eq!(packet.header.flag, Flag::Error);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7]);
        }

        #[test]
        fn not_aligned_to_block() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                11, 13, 17, //data
                4 ^ 4 ^ 11, 5 ^ 1 ^ 5 ^ 13, 1 ^ 2 ^ 6 ^ 17, 8 ^ 3 ^ 7
            ];
            let packet = DataPacket::from(&data.as_slice(), 4).unwrap();
            assert_eq!(packet.header.id, 1 << 8);
            assert_eq!(packet.header.seq, 5);
            assert_eq!(packet.header.ack, 8);
            assert_eq!(packet.header.flag, Flag::Error);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17]);
        }

        #[test]
        fn checksum_not_match() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                4 ^ 4, 5 ^ 1 ^ 5, /*1 ^*/ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            let packet = DataPacket::from(&data.as_slice(), 4).err().unwrap();
            assert_eq!("Checksum doesn't match", packet);
        }

        #[test]
        fn data_not_match() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                /*1*/0, 2, 3, //data
                4, 5, 6, 7, //data
                4 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            let packet = DataPacket::from(&data.as_slice(), 4).err().unwrap();
            assert_eq!("Checksum doesn't match", packet);
        }

        #[test]
        fn data_too_short() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                // no data
                4 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6/*, 8 ^ 3 ^ 7*/
            ];
            let packet = DataPacket::from(&data.as_slice(), 4).err().unwrap();
            assert_eq!("Not enough data received", packet);
        }

        #[test]
        fn without_checksum() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, //data
            ];
            let packet = DataPacket::from(&data.as_slice(), 0).unwrap();
            assert_eq!(packet.header.id, 1 << 8);
            assert_eq!(packet.header.seq, 5);
            assert_eq!(packet.header.ack, 8);
            assert_eq!(packet.header.flag, Flag::Error);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6]);
        }
    }

    mod to_binary {
        use crate::packet::DataPacket;
        use crate::packet::PacketHeader;
        use crate::packet::Flag;

        #[test]
        fn valid_transfer() {
            let packet = DataPacket {
                header: PacketHeader {
                    id: 1 << 8,
                    seq: 5,
                    ack: 8,
                    flag: Flag::Error,
                },
                data: vec![1, 2, 3, 4, 5, 6, 7],
            };
            let mut actual = vec![0; 20];
            let wrote = packet.to_bin_with_checksum(4, &mut actual);
            let expected: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                4 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            assert_eq!(wrote, expected.len());
            assert_eq!(actual, expected);
        }

        #[test]
        fn not_aligned_to_block() {
            let packet = DataPacket {
                header: PacketHeader {
                    id: 1 << 8,
                    seq: 5,
                    ack: 8,
                    flag: Flag::Error,
                },
                data: vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17],
            };
            let mut actual = vec![0; 23];
            let wrote = packet.to_bin_with_checksum(4, &mut actual);
            let expected: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                11, 13, 17, //data
                4 ^ 4 ^ 11, 5 ^ 1 ^ 5 ^ 13, 1 ^ 2 ^ 6 ^ 17, 8 ^ 3 ^ 7
            ];
            assert_eq!(wrote, expected.len());
            assert_eq!(actual, expected);
        }

        #[test]
        fn no_checksum() {
            let packet = DataPacket {
                header: PacketHeader {
                    id: 1 << 8,
                    seq: 5,
                    ack: 8,
                    flag: Flag::Error,
                },
                data: vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17],
            };
            let mut actual = vec![0; 19];
            let wrote = packet.to_bin_with_checksum(0, &mut actual);
            let expected: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                11, 13, 17, //data
            ];
            assert_eq!(wrote, expected.len());
            assert_eq!(actual, expected);
        }
    }

    mod flag_deserialization {
        use crate::packet::DataPacket;
        use crate::packet::Flag;

        #[test]
        fn invalid_flag() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                7, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                7 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            let packet = DataPacket::from(&data.as_slice(), 4).unwrap();
            assert_eq!(packet.header.id, 1 << 8);
            assert_eq!(packet.header.seq, 5);
            assert_eq!(packet.header.ack, 8);
            assert_eq!(packet.header.flag, Flag::None);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7]);
        }
    }
}
