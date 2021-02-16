use super::{ToBin, Flag, ParsingError, PacketHeader};

#[derive(Debug)]
pub struct ErrorPacket {
    pub header: PacketHeader,
}

impl ToBin for ErrorPacket {
    fn bin_size(&self) -> usize {
        return self.header.bin_size();
    }

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        return self.header.to_bin_buff(buff);
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError> {
        Ok(Self {
            header: PacketHeader::from_bin(memory)?,
        })
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
