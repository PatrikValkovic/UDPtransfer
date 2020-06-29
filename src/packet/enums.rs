#[derive(Debug, PartialEq)]
pub enum ParsingError {
    InvalidSize(usize, usize), // expected, actual
    ChecksumNotMatch,
    InvalidFlag(u8),
}

pub trait ToBin: Sized {
    fn bin_size(&self) -> usize;

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize;

    fn to_bin(&self) -> Vec<u8> {
        let mut vect = vec![0; self.bin_size()];
        self.to_bin_buff(vect.as_mut_slice());
        return vect;
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError>;
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
    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        buff[0] = self.value();
        return 1;
    }
    fn from_bin(val: &[u8]) -> Result<Self, ParsingError> {
        Ok(match val[0] {
            0x1 => Flag::Init,
            0x2 => Flag::Data,
            0x4 => Flag::Error,
            0x8 => Flag::End,
            _ => Flag::None,
        })
    }
}

impl Flag {
    pub fn value(&self) -> u8 {
        match self {
            Flag::None => 0x0,
            Flag::Init => 0x1,
            Flag::Data => 0x2,
            Flag::Error => 0x4,
            Flag::End => 0x8,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::packet::{Packet, ParsingError};

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
        if let Err(ParsingError::InvalidFlag(7)) = Packet::from_bin(&data.as_slice(), 4) {} else {
            panic!();
        }
    }
}