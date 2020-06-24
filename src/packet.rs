use std::vec::Vec;
use byteorder::{NetworkEndian, ByteOrder};

trait ToBin {
    fn bin_size(&self) -> usize;

    fn to_bin_buff(&self, buff: &mut [u8]);

    fn to_bin(&self) -> Vec<u8> {
        let mut vect = vec![0; self.bin_size()];
        self.to_bin_buff(vect.as_mut_slice());
        return vect;
    }

    fn from_bin(memory: &[u8]) -> Self;
}


#[derive(Debug, PartialEq)]
pub enum Flag {
    None,
    Init,
    Data,
    Error,
    End,
}

impl ToBin for Flag {
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


#[derive(Debug)]
pub struct PacketHeader {
    pub id: u32,
    pub seq: u16,
    pub ack: u16,
    pub flag: Flag,
}

impl ToBin for PacketHeader {
    fn bin_size(&self) -> usize {
        Self::bin_size()
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

impl PacketHeader {
    pub fn bin_size() -> usize {
        return 9;
    }
    pub fn flag_position() -> usize {
        return 8;
    }
}


#[derive(Debug)]
pub struct InitPacket {
    pub header: PacketHeader,
    pub window_size: u16,
    pub packet_size: u16,
    pub checksum_size: u16,
}

impl ToBin for InitPacket {
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
            checksum_size,
        };
    }
}

impl From<(u16, u16, u16)> for InitPacket {
    fn from((window_size, packet_size, checksum_size): (u16, u16, u16)) -> Self {
        Self::new(window_size, packet_size, checksum_size)
    }
}


#[derive(Debug)]
pub struct DataPacket {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

impl ToBin for DataPacket {
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
                flag: Flag::Data,
            },
            data,
        };
    }
}

impl From<(&[u8], usize)> for DataPacket {
    fn from((data, checksum_size): (&[u8], usize)) -> Self {
        return DataPacket::from_bin(&data[..data.len() - checksum_size]);
    }
}

impl From<(Vec<u8>, u32, u16, u16)> for DataPacket {
    fn from((data, connection_id, seq, ack): (Vec<u8>, u32, u16, u16)) -> Self {
        Self::new(data, connection_id, seq, ack)
    }
}


#[derive(Debug)]
pub struct ErrorPacket {
    header: PacketHeader,
}

impl ToBin for ErrorPacket {
    fn bin_size(&self) -> usize {
        return self.header.bin_size();
    }

    fn to_bin_buff(&self, buff: &mut [u8]) {
        return self.header.to_bin_buff(buff);
    }

    fn from_bin(memory: &[u8]) -> Self {
        return Self {
            header: PacketHeader::from_bin(memory),
        };
    }
}

impl ErrorPacket {
    pub fn new(connection_id: u32) -> Self {
        return Self {
            header: PacketHeader {
                id: connection_id,
                seq: 0,
                ack: 0,
                flag: Flag::Error,
            },
        };
    }
}

impl From<u32> for ErrorPacket {
    fn from(connection_id: u32) -> Self {
        return Self::new(connection_id);
    }
}


#[derive(Debug)]
pub struct EndPacket {
    header: PacketHeader,
}

impl ToBin for EndPacket {
    fn bin_size(&self) -> usize {
        return self.header.bin_size();
    }

    fn to_bin_buff(&self, buff: &mut [u8]) {
        return self.header.to_bin_buff(buff);
    }

    fn from_bin(memory: &[u8]) -> Self {
        return Self {
            header: PacketHeader::from_bin(memory),
        };
    }
}

impl EndPacket {
    pub fn new(connection_id: u32, seq_num: u16) -> Self {
        return Self {
            header: PacketHeader {
                id: connection_id,
                seq: seq_num,
                ack: seq_num,
                flag: Flag::End,
            },
        };
    }
}

impl From<(u32, u16)> for EndPacket {
    fn from((connection_id, seq_num): (u32, u16)) -> Self {
        return Self::new(connection_id, seq_num);
    }
}


#[derive(Debug)]
pub enum Packet {
    Init(InitPacket),
    Data(DataPacket),
    Error(ErrorPacket),
    End(EndPacket),
}

impl ToBin for Packet {
    fn bin_size(&self) -> usize {
        match self {
            Self::Init(x) => x.bin_size(),
            Self::Data(x) => x.bin_size(),
            Self::Error(x) => x.bin_size(),
            Self::End(x) => x.bin_size(),
        }
    }

    fn to_bin_buff(&self, buff: &mut [u8]) {
        match self {
            Self::Init(x) => x.to_bin_buff(buff),
            Self::Data(x) => x.to_bin_buff(buff),
            Self::Error(x) => x.to_bin_buff(buff),
            Self::End(x) => x.to_bin_buff(buff),
        }
    }

    fn from_bin(memory: &[u8]) -> Self {
        let flag_pos = PacketHeader::flag_position();
        let flag = Flag::from_bin(&memory[flag_pos..flag_pos + 1]);
        match flag {
            Flag::Init => Self::Init(InitPacket::from_bin(memory)),
            Flag::Error => Self::Error(ErrorPacket::from_bin(memory)),
            Flag::End => Self::End(EndPacket::from_bin(memory)),
            Flag::Data => Self::Data(DataPacket::from_bin(memory)),
            Flag::None => panic!("Received invalid flag"), //TODO better
        }
    }
}

//TODO implement rest of method for Packet


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

fn checksums_match(first: &[u8], second: &[u8]) -> bool {
    if let Some(_) = first.iter()
        .zip(second.iter())
        .find(|(&comp, &inside)| { comp != inside }) {
        return false;
    } else {
        return true;
    }
}

fn from_bin_checksum<T>(memory: &[u8], checksum: usize) -> T
    where T: ToBin {
    if checksum + PacketHeader::bin_size() > memory.len() {
        panic!("Not enough data received") //TODO better
    }

    let checksum_start = memory.len() - checksum;
    if checksum > 0 {
        let orig_checksum = Vec::from(&memory[checksum_start..]);
        let comp_checksum = construct_checksum(&memory[..checksum_start], checksum);
        if !checksums_match(&orig_checksum, &comp_checksum) {
            panic!("Checksum doesn't match") //TODO better
        }
    }
    return T::from_bin(&memory[..checksum_start]);
}

fn to_bin_buff_checksum<T>(obj: &T, memory: &mut [u8], checksum: usize) -> usize
    where T: ToBin
{
    let data_end = obj.bin_size();
    let packet_size = data_end + checksum;
    debug_assert!(memory.len() >= packet_size);

    obj.to_bin_buff(&mut memory[..data_end]);

    if checksum > 0 {
        let checksum = construct_checksum(&memory[..data_end], checksum);
        memory[data_end..].copy_from_slice(checksum.as_slice());
    }

    return packet_size;
}

fn to_bin_checksum<T>(obj: &T, checksum: usize) -> Vec<u8>
    where T: ToBin
{
    let mut memory = vec![0; obj.bin_size() + checksum];
    to_bin_buff_checksum(obj, &mut memory, checksum);
    return memory;
}


#[cfg(test)]
mod tests {
    mod new {
        use crate::packet::{DataPacket, Flag, from_bin_checksum};

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
            let packet = from_bin_checksum::<DataPacket>(&data.as_slice(), 4);
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
            let packet = from_bin_checksum::<DataPacket>(&data.as_slice(), 4);
            assert_eq!(packet.header.id, 1 << 8);
            assert_eq!(packet.header.seq, 5);
            assert_eq!(packet.header.ack, 8);
            assert_eq!(packet.header.flag, Flag::Error);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17]);
        }

        #[test]
        #[should_panic(expected = "Checksum doesn't match")]
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
            let packet = from_bin_checksum::<DataPacket>(&data.as_slice(), 4);
        }

        #[test]
        #[should_panic(expected = "Checksum doesn't match")]
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
            let packet = from_bin_checksum::<DataPacket>(&data.as_slice(), 4);
        }

        #[test]
        #[should_panic(expected = "Not enough data received")]
        fn data_too_short() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                // no data
                4 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6/*, 8 ^ 3 ^ 7*/
            ];
            let packet = from_bin_checksum::<DataPacket>(&data.as_slice(), 4);
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
            let packet = from_bin_checksum::<DataPacket>(&data.as_slice(), 0);
            assert_eq!(packet.header.id, 1 << 8);
            assert_eq!(packet.header.seq, 5);
            assert_eq!(packet.header.ack, 8);
            assert_eq!(packet.header.flag, Flag::Error);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6]);
        }
    }

    mod to_binary {
        use crate::packet::{DataPacket, PacketHeader, Flag, to_bin_checksum, to_bin_buff_checksum};

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
            let wrote = to_bin_buff_checksum(&packet, &mut actual, 4);
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
            let wrote = to_bin_buff_checksum(&packet, &mut actual, 4);
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
            let wrote = to_bin_buff_checksum(&packet, &mut actual, 0);
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
        use crate::packet::{DataPacket, from_bin_checksum};
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
            let packet = from_bin_checksum::<DataPacket>(&data.as_slice(), 4);
            assert_eq!(packet.header.id, 1 << 8);
            assert_eq!(packet.header.seq, 5);
            assert_eq!(packet.header.ack, 8);
            assert_eq!(packet.header.flag, Flag::None);
            assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7]);
        }
    }
}
