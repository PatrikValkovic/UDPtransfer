use crate::packet::enums::ParsingError::InvalidFlag;

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
        match val[0] {
            0x0 => Ok(Flag::None),
            0x1 => Ok(Flag::Init),
            0x2 => Ok(Flag::Data),
            0x4 => Ok(Flag::Error),
            0x8 => Ok(Flag::End),
            _ => Err(InvalidFlag(val[0])),
        }
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
    use crate::packet::{Flag, ParsingError, ToBin};

    #[test]
    fn valid_flag() {
        let data: Vec<u8> = vec![0x4];
        if let Ok(Flag::Error) = Flag::from_bin(&data) {} else {
            panic!();
        }
    }

    #[test]
    fn invalid_flag() {
        let data: Vec<u8> = vec![7];
        if let Err(ParsingError::InvalidFlag(7)) = Flag::from_bin(&data) {} else {
            panic!();
        }
    }
}