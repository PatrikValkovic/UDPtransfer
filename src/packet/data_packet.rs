use super::{ToBin, Flag, ParsingError, PacketHeader};

#[derive(Debug)]
pub struct DataPacket {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

impl ToBin for DataPacket {
    fn bin_size(&self) -> usize {
        return self.header.bin_size() + self.data.len();
    }

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        let header_size = self.header.bin_size();
        let header_wrote = self.header.to_bin_buff(buff);
        buff[header_size..].copy_from_slice(self.data.as_slice());
        return header_wrote + self.data.len();
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError> {
        let header = PacketHeader::from_bin(memory)?;
        let header_size = header.bin_size();
        let data = Vec::from(&memory[header_size..]);

        Ok(Self {
            header,
            data,
        })
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

impl From<(Vec<u8>, u32, u16, u16)> for DataPacket {
    fn from((data, connection_id, seq, ack): (Vec<u8>, u32, u16, u16)) -> Self {
        Self::new(data, connection_id, seq, ack)
    }
}

#[cfg(test)]
mod tests {}
