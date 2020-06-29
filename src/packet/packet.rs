use super::{ToBin, Flag, ParsingError, PacketHeader};
use super::{InitPacket, DataPacket, ErrorPacket, EndPacket};
use std::num::ParseIntError;


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

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        match self {
            Self::Init(x) => x.to_bin_buff(buff),
            Self::Data(x) => x.to_bin_buff(buff),
            Self::Error(x) => x.to_bin_buff(buff),
            Self::End(x) => x.to_bin_buff(buff),
        }
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError> {
        let flag_pos = PacketHeader::flag_position();
        let flag = Flag::from_bin(&memory[flag_pos..flag_pos + 1]).unwrap();
        Ok(match flag {
            Flag::Init => Self::Init(InitPacket::from_bin(memory)?),
            Flag::Error => Self::Error(ErrorPacket::from_bin(memory)?),
            Flag::End => Self::End(EndPacket::from_bin(memory)?),
            Flag::Data => Self::Data(DataPacket::from_bin(memory)?),
            Flag::None => return Err(ParsingError::InvalidFlag(memory[flag_pos])),
        })
    }
}

impl Packet {
    pub fn bin_size(&self) -> usize {
        return ToBin::bin_size(self);
    }

    pub fn to_bin(&self, checksum: usize) -> Vec<u8> {
        let mut memory = vec![0; self.bin_size() + checksum];
        self.to_bin_buff(&mut memory, checksum);
        return memory;
    }

    pub fn to_bin_buff(&self, memory: &mut [u8], checksum: usize) -> usize {
        let data_end = self.bin_size();
        let packet_size = data_end + checksum;
        debug_assert!(memory.len() >= packet_size);

        ToBin::to_bin_buff(self, &mut memory[..data_end]);

        if checksum > 0 {
            let checksum = construct_checksum(&memory[..data_end], checksum);
            memory[data_end..].copy_from_slice(checksum.as_slice());
        }

        return packet_size;
    }

    pub fn from_bin(memory: &[u8], checksum: usize) -> Result<Self, ParsingError> {
        if checksum + PacketHeader::bin_size() > memory.len() {
            return Err(ParsingError::InvalidSize(checksum + PacketHeader::bin_size(), memory.len()));
        }
        let checksum_start = memory.len() - checksum;

        let package = match ToBin::from_bin(&memory[..checksum_start]) {
            Ok(packet) => packet,
            Err(ParsingError::InvalidSize(expected, actual)) => return Err(ParsingError::InvalidSize(expected, memory.len())),
            Err(e) => return Err(e),
        };

        if checksum > 0 {
            let orig_checksum = Vec::from(&memory[checksum_start..]);
            let comp_checksum = construct_checksum(&memory[..checksum_start], checksum);
            if !checksums_match(&orig_checksum, &comp_checksum) {
                return Err(ParsingError::ChecksumNotMatch);
            }
        }

        return Ok(package);
    }
}

impl From<InitPacket> for Packet {
    fn from(packet: InitPacket) -> Self {
        Packet::Init(packet)
    }
}

impl From<DataPacket> for Packet {
    fn from(packet: DataPacket) -> Self {
        Packet::Data(packet)
    }
}

impl From<ErrorPacket> for Packet {
    fn from(packet: ErrorPacket) -> Self {
        Packet::Error(packet)
    }
}

impl From<EndPacket> for Packet {
    fn from(packet: EndPacket) -> Self {
        Packet::End(packet)
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

fn checksums_match(first: &[u8], second: &[u8]) -> bool {
    if let Some(_) = first.iter()
        .zip(second.iter())
        .find(|(&comp, &inside)| { comp != inside }) {
        return false;
    } else {
        return true;
    }
}


#[cfg(test)]
mod tests {
    mod from_binary {
        use crate::packet::{Packet, Flag, ParsingError};

        #[test]
        fn should_parse_successfully() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                2, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                2 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            match Packet::from_bin(&data.as_slice(), 4) {
                Ok(Packet::Data(packet)) => {
                    assert_eq!(packet.header.id, 1 << 8);
                    assert_eq!(packet.header.seq, 5);
                    assert_eq!(packet.header.ack, 8);
                    assert_eq!(packet.header.flag, Flag::Data);
                    assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7]);
                }
                rest => panic!("{:?}", rest),
            }
        }

        #[test]
        fn not_aligned_to_block() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                2, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                11, 13, 17, //data
                2 ^ 4 ^ 11, 5 ^ 1 ^ 5 ^ 13, 1 ^ 2 ^ 6 ^ 17, 8 ^ 3 ^ 7
            ];
            if let Ok(Packet::Data(packet)) = Packet::from_bin(&data.as_slice(), 4) {
                assert_eq!(packet.header.id, 1 << 8);
                assert_eq!(packet.header.seq, 5);
                assert_eq!(packet.header.ack, 8);
                assert_eq!(packet.header.flag, Flag::Data);
                assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17]);
            } else {
                panic!();
            }
        }

        #[test]
        fn checksum_not_match() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                2, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                2 ^ 4, 5 ^ 1 ^ 5, /*1 ^*/ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            if let Err(ParsingError::ChecksumNotMatch) = Packet::from_bin(&data.as_slice(), 4) {} else {
                panic!("Test failed");
            }
        }

        #[test]
        fn data_not_match() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                2, //flag
                /*1*/0, 2, 3, //data
                4, 5, 6, 7, //data
                2 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            if let Err(ParsingError::ChecksumNotMatch) = Packet::from_bin(&data.as_slice(), 4) {} else {
                panic!("Test failed");
            }
        }

        #[test]
        fn data_too_short() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                2, //flag
                // no data
                2 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6/*, 8 ^ 3 ^ 7*/
            ];
            if let Err(ParsingError::InvalidSize(_, _)) = Packet::from_bin(&data.as_slice(), 4) {} else {
                panic!("Test failed");
            }
        }

        #[test]
        fn without_checksum() {
            let data: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                2, //flag
                1, 2, 3, //data
                4, 5, 6, //data
            ];
            if let Ok(Packet::Data(packet)) = Packet::from_bin(&data.as_slice(), 0) {
                assert_eq!(packet.header.id, 1 << 8);
                assert_eq!(packet.header.seq, 5);
                assert_eq!(packet.header.ack, 8);
                assert_eq!(packet.header.flag, Flag::Data);
                assert_eq!(packet.data, vec![1, 2, 3, 4, 5, 6]);
            } else {
                panic!();
            }
        }
    }

    mod to_binary {
        use crate::packet::{DataPacket, PacketHeader, Flag, Packet};

        #[test]
        fn valid_transfer() {
            let packet = Packet::from(DataPacket {
                header: PacketHeader {
                    id: 1 << 8,
                    seq: 5,
                    ack: 8,
                    flag: Flag::Error,
                },
                data: vec![1, 2, 3, 4, 5, 6, 7],
            });
            let mut actual = vec![0; 20];
            packet.to_bin_buff(&mut actual, 4);
            let expected: Vec<u8> = vec![
                0, 0, 1, 0, //id
                0, 5, //seq
                0, 8, //ack
                4, //flag
                1, 2, 3, //data
                4, 5, 6, 7, //data
                4 ^ 4, 5 ^ 1 ^ 5, 1 ^ 2 ^ 6, 8 ^ 3 ^ 7
            ];
            assert_eq!(actual, expected);
        }

        #[test]
        fn not_aligned_to_block() {
            let packet = Packet::from(DataPacket {
                header: PacketHeader {
                    id: 1 << 8,
                    seq: 5,
                    ack: 8,
                    flag: Flag::Error,
                },
                data: vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17],
            });
            let mut actual = vec![0; 23];
            packet.to_bin_buff(&mut actual, 4);
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
            assert_eq!(actual, expected);
        }

        #[test]
        fn no_checksum() {
            let packet = Packet::from(DataPacket {
                header: PacketHeader {
                    id: 1 << 8,
                    seq: 5,
                    ack: 8,
                    flag: Flag::Error,
                },
                data: vec![1, 2, 3, 4, 5, 6, 7, 11, 13, 17],
            });
            let mut actual = vec![0; 19];
            let wrote = packet.to_bin_buff(&mut actual, 0);
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
}
