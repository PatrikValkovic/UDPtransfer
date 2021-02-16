use crate::packet::{ToBin, ParsingError};

pub struct Checksum {
    size: usize,
    checksum: Vec<u8>,
}

impl ToBin for Checksum {
    fn bin_size(&self) -> usize {
        self.size
    }

    fn to_bin_buff(&self, buff: &mut [u8]) -> usize {
        buff.copy_from_slice(&self.checksum);
        return self.bin_size();
    }

    fn from_bin(memory: &[u8]) -> Result<Self, ParsingError> {
        Ok(Self::from(memory))
    }
}

impl From<&[u8]> for Checksum {
    fn from(buffer: &[u8]) -> Self {
        Self {
            checksum: Vec::from(buffer),
            size: buffer.len()
        }
    }
}
impl Checksum {
    pub fn from_packet_content(packet_buffer: &[u8], checksum_size: usize) -> Self {
        let mut buffer = vec![0; checksum_size];

        if checksum_size > 0 {
            for current_block in 0..packet_buffer.len() / checksum_size + 1 {
                for current_byte in 0..checksum_size {
                    if current_block * checksum_size + current_byte < packet_buffer.len() {
                        buffer[current_byte] ^= packet_buffer[current_block * checksum_size + current_byte];
                    }
                    else {
                        break;
                    }
                }
            }
        }

        Self {
            size: checksum_size,
            checksum: buffer
        }
    }

    pub fn is_same(&self, second: &Self) -> bool {
        return self.size == second.size && self.checksum == second.checksum;
    }
}


#[cfg(test)]
mod tests {
    use crate::packet::{Checksum};

    #[test]
    fn should_get_from_buffer() {
        let data = vec![0x1, 0x2, 0x3];
        let checksum = Checksum::from(data.as_slice());
        assert_eq!(checksum.size, 3);
        assert_eq!(checksum.checksum, data);
    }

    #[test]
    fn should_create_from_buffer() {
        let data = vec![0x1, 0x2, 0x8];
        let checksum = Checksum::from_packet_content(&data, 1);
        assert_eq!(checksum.size, 1);
        assert_eq!(checksum.checksum.len(), 1);
        assert_eq!(checksum.checksum[0], 0xB);
    }

    #[test]
    fn should_create_zero_length() {
        let data = vec![0x1, 0x2, 0x8];
        let checksum = Checksum::from_packet_content(&data, 0);
        assert_eq!(checksum.size, 0);
        assert_eq!(checksum.checksum.len(), 0);
    }


    #[test]
    fn should_create_not_aligned() {
        let data = vec![0x1, 0x2, 0x8];
        let expected = vec![0x1 ^ 0x8, 0x2];
        let checksum = Checksum::from_packet_content(&data, 2);
        assert_eq!(checksum.size, 2);
        assert_eq!(checksum.checksum.len(), 2);
        assert_eq!(checksum.checksum, expected);
    }
}