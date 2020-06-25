use super::{ToBin, Flag, ParsingError, PacketHeader};

#[derive(Debug)]
pub struct EndPacket {
    header: PacketHeader,
}

impl ToBin for EndPacket {
    fn bin_size(&self) -> usize {
        return self.header.bin_size();
    }

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        return self.header.to_bin_buff(buff);
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError> {
        Ok(Self {
            header: PacketHeader::from_bin(memory).unwrap(),
        })
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
