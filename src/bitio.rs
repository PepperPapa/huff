pub struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    nbits: u8,
}

impl BitWriter {
    #[must_use]
    pub fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            current: 0,
            nbits: 0,
        }
    }
    pub fn write_bit(&mut self, bit: bool) {
        if bit {
            self.current |= 1u8 << (7 - self.nbits);
        }
        self.nbits += 1;
        if self.nbits == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.nbits = 0;
        }
    }
    pub fn write_bits(&mut self, bits: &[bool]) {
        for &b in bits {
            self.write_bit(b);
        }
    }

    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.bytes.push(self.current);
        }
        self.bytes
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }
    pub fn read_bit(&mut self) -> Option<bool> {
        if self.byte_pos >= self.data.len() {
            return None;
        }
        let byte = self.data[self.byte_pos];
        let bit = (byte >> (7 - self.bit_pos)) & 1 == 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Some(bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bits() {
        let bits = [true, false, true, true, false, false, true, false, true];
        let mut w = BitWriter::new();
        w.write_bits(&bits);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        for &expected in &bits {
            assert_eq!(r.read_bit(), Some(expected));
        }
    }

    #[test]
    fn empty_writer_produces_empty_vec() {
        let w = BitWriter::new();
        assert_eq!(w.finish(), Vec::<u8>::new());
    }

    #[test]
    fn single_bit_padded_to_msb() {
        let mut w = BitWriter::new();
        w.write_bit(true);
        assert_eq!(w.finish(), vec![0b1000_0000]);
    }

    #[test]
    fn read_beyond_end_returns_none() {
        let mut w = BitWriter::new();
        w.write_bit(true);
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        // 1 个有效位 + 7 个 padding，共 8 次 Some
        for _ in 0..8 {
            assert!(r.read_bit().is_some());
        }
        assert_eq!(r.read_bit(), None);
    }

    #[test]
    fn byte_boundary() {
        // 正好 8 位，不应产生额外 padding 字节
        let mut w = BitWriter::new();
        for _ in 0..8 {
            w.write_bit(true);
        }
        assert_eq!(w.finish(), vec![0xFF]);
    }
}
