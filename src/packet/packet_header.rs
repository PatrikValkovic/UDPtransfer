use byteorder::{NetworkEndian, ByteOrder};
use super::{ToBin, Flag, ParsingError};

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

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        debug_assert!(buff.len() >= Self::bin_size());
        NetworkEndian::write_u32(&mut buff[..4], self.id);
        NetworkEndian::write_u16(&mut buff[4..6], self.seq);
        NetworkEndian::write_u16(&mut buff[6..8], self.ack);
        return 8 + self.flag.to_bin_buff(&mut buff[8..9]);
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError> {
        debug_assert!(memory.len() >= Self::bin_size());
        let id = NetworkEndian::read_u32(&memory[..4]);
        let seq = NetworkEndian::read_u16(&memory[4..6]);
        let ack = NetworkEndian::read_u16(&memory[6..8]);
        let flag = Flag::from_bin(&memory[8..9])?;
        Ok(PacketHeader {
            id,
            seq,
            ack,
            flag,
        })
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
