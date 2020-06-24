
pub trait Transferable {
    fn bin_size(&self) -> usize;

    fn to_bin_buff(&self, buff: &mut [u8]);

    fn to_bin(&self) -> Vec<u8> {
        let mut vect = vec![0; self.bin_size()];
        self.to_bin_buff(vect.as_mut_slice());
        return vect;
    }

    fn from_bin(memory: &[u8]) -> Self;
}