use byteorder::{NetworkEndian, ByteOrder};
use super::{ToBin, Flag, ParsingError, PacketHeader};

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
        return (self.packet_size - self.checksum_size) as usize;
    }

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        debug_assert!(buff.len() >= self.bin_size());
        let header_size = self.header.bin_size() as usize;

        self.header.to_bin_buff(buff);
        NetworkEndian::write_u16(&mut buff[header_size..header_size + 2], self.window_size);
        NetworkEndian::write_u16(&mut buff[header_size + 2..header_size + 4], self.packet_size);
        NetworkEndian::write_u16(&mut buff[header_size + 4..header_size + 6], self.checksum_size);

        let checksum_start = (self.packet_size - self.checksum_size) as usize;
        for val in &mut buff[header_size+6..checksum_start] {
            *val = 0;
        }

        return checksum_start as usize;
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError> {
        let header = PacketHeader::from_bin(memory).unwrap();
        let header_size = header.bin_size() as usize;
        let window_size = NetworkEndian::read_u16(&memory[header_size..header_size + 2]);
        let packet_size = NetworkEndian::read_u16(&memory[header_size + 2..header_size + 4]);
        let checksum_size = NetworkEndian::read_u16(&memory[header_size + 4..header_size + 6]);

        let expected_memory = (packet_size - checksum_size) as usize;
        if memory.len() < expected_memory {
            return Err(ParsingError::InvalidSize(expected_memory, memory.len()));
        }

        Ok(InitPacket {
            header,
            window_size,
            packet_size,
            checksum_size,
        })
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

#[cfg(test)]
mod tests {
    use crate::packet::{Packet, InitPacket, Flag, enums::ToBin, ParsingError};

    #[test]
    fn to_binary() {
        let packet = Packet::from(InitPacket::new(0x8, 0x32, 0x4));
        let bin = packet.to_bin(0x4);
        let expect = vec![
            0, 0, 0, 0, //id
            0, 0, 0, 0, //seq ack
            Flag::to_bin(&Flag::Init)[0],
            0, 0x8, 0, 0x32, 0, 0x4,
            0, 0, 0, 0, 0,  //data byte20
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 30
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 40
            0, 0, 0, 0, 0, 0,  //data byte 46
            Flag::to_bin(&Flag::Init)[0] ^ 0x32, 0, 0x8 ^ 0x4, 0 //checksum
        ];
        assert_eq!(bin, expect);
    }

    #[test]
    fn from_binary() {
        let data = vec![
            0, 0x64, 0, 0, //id
            0, 0, 0, 0, //seq ack
            Flag::to_bin(&Flag::Init)[0],
            0, 0x8, 0, 0x32, 0, 0x4,
            0, 0, 0, 0, 0,  //data byte20
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 30
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 40
            0, 0, 0, 0, 0, 0,  //data byte 46
            Flag::to_bin(&Flag::Init)[0] ^ 0x32, 0x64, 0x8 ^ 0x4, 0 //checksum
        ];
        match Packet::from_bin(&data, 4) {
            Ok(Packet::Init(x)) => {
                assert_eq!(x.header.id, 0x64 << 16);
                assert_eq!(x.header.seq, 0);
                assert_eq!(x.header.ack, 0);
                assert_eq!(x.header.flag, Flag::Init);
                assert_eq!(x.window_size, 0x8);
                assert_eq!(x.packet_size, 0x32);
                assert_eq!(x.checksum_size, 0x4);
            }
            _ => panic!()
        };
    }

    #[test]
    fn wrong_checksum() {
        let data = vec![
            0, 0x64, 0, 0, //id
            0, 0, 0, 0, //seq ack
            Flag::to_bin(&Flag::Init)[0],
            0, 0x8, 0, 0x32, 0, 0x4,
            0, 0, 0, 0, 0,  //data byte20
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 30
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 40
            0, 0, 0, 0, 0, 0,  //data byte 46
            Flag::to_bin(&Flag::Init)[0] ^ 0x32, 0 /*0x64*/, 0x8 ^ 0x4, 0 //checksum
        ];
        if let Err(ParsingError::ChecksumNotMatch) = Packet::from_bin(&data, 4) {} else {
            panic!()
        };
    }

    #[test]
    fn wrong_length() {
        let data = vec![
            0, 0x64, 0, 0, //id
            0, 0, 0, 0, //seq ack
            Flag::to_bin(&Flag::Init)[0],
            0, 0x8, 0, 0x32, 0, 0x4,
            0, 0, 0, 0, 0,  //data byte20
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 30
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  //data byte 40
        ];

        match Packet::from_bin(&data, 4) {
            Err(e) => println!("Err: {:?}", e),
            _ => ()
        };

        if let Err(ParsingError::InvalidSize(_, _)) = Packet::from_bin(&data, 4) {} else {
            panic!()
        };
    }
}
