use super::{ToBin, Flag, ParsingError, PacketHeader, Checksum};
use super::{InitPacket, DataPacket, ErrorPacket, EndPacket};

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
        let flag = Flag::from_bin(&memory[flag_pos..flag_pos + 1])?;
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

    pub fn to_bin_buff(&self, memory: &mut [u8], checksum_size: usize) -> usize {
        let data_end = self.bin_size();
        let packet_size = data_end + checksum_size;
        debug_assert!(memory.len() >= packet_size);

        ToBin::to_bin_buff(self, &mut memory[..data_end]);

        let checksum = Checksum::from_packet_content(&memory[..data_end], checksum_size);
        checksum.to_bin_buff(&mut memory[data_end..data_end+checksum_size]);

        return packet_size;
    }

    pub fn from_bin(memory: &[u8], checksum: usize) -> Result<Self, ParsingError> {
        if checksum + PacketHeader::bin_size() > memory.len() {
            return Err(ParsingError::InvalidSize(checksum + PacketHeader::bin_size(), memory.len()));
        }
        let checksum_start = memory.len() - checksum;

        let package = match ToBin::from_bin(&memory[..checksum_start]) {
            Ok(packet) => packet,
            Err(ParsingError::InvalidSize(expected, _)) => return Err(ParsingError::InvalidSize(expected+checksum, memory.len())),
            Err(e) => return Err(e),
        };

        let stored_checksum = Checksum::from_bin(&memory[checksum_start..])?;
        let computed_checksum = Checksum::from_packet_content(&memory[..checksum_start], checksum);
        if !stored_checksum.is_same(&computed_checksum){
                return Err(ParsingError::ChecksumNotMatch);
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
