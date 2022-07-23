// https://www.matthewflickinger.com/lab/whatsinagif/scripts/data_helpers.js
pub struct BitReader {
    bytes: Vec<u8>,
    byte_offset: usize,
    bit_offset: usize,
}
impl BitReader {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            byte_offset: 0,
            bit_offset: 0,
        }
    }
    fn shl_or(&mut self, val: u16, shift: usize, def: u16) -> u16 {
        [val << (shift & 15), def][((shift & !15) != 0) as usize]
    }
    fn shr_or(&mut self, val: u16, shift: usize, def: u16) -> u16 {
        [val >> (shift & 15), def][((shift & !15) != 0) as usize]
    }
    pub fn read_bits(&mut self, len: usize) -> Option<u16> {
        let mut result = 0;
        let mut rbits: usize = 0;
        while rbits < len {
            if self.byte_offset >= self.bytes.len() {
                println!(
                    "Not enough bytes to read {} bits (read {} bits) --> {}:{}:{}",
                    len,
                    rbits,
                    file!(),
                    line!(),
                    column!(),
                );
                return None;
            }
            let bbits = std::cmp::min(8 - self.bit_offset, len - rbits);

            let temp = self.shr_or(0xFF, 8 - bbits, 0);
            let mask = self.shl_or(temp, self.bit_offset, 0);

            let temp = self.shr_or(
                self.bytes[self.byte_offset] as u16 & mask,
                self.bit_offset,
                0,
            );
            result += self.shl_or(temp, rbits, 0);
            rbits += bbits;
            self.bit_offset += bbits;
            if self.bit_offset == 8 {
                self.byte_offset += 1;
                self.bit_offset = 0;
            }
        }
        Some(result)
    }

    pub fn has_bits(&mut self, len: usize) -> Option<bool> {
        if len > 12 {
            println!(
                "Exceeds max bit size: ${} (max: 12) --> {}:{}:{}",
                len,
                file!(),
                line!(),
                column!(),
            );
            return None;
        }
        if self.byte_offset >= self.bytes.len() {
            return Some(false);
        }
        let bits_remain = 8 - self.bit_offset;
        if len <= bits_remain {
            return Some(true);
        }
        let bytes_remain = self.bytes.len() - self.byte_offset - 1;
        if bytes_remain < 1 {
            return Some(false);
        }
        if len > bits_remain + 8 * bytes_remain {
            return Some(false);
        }
        return Some(true);
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        match self.has_bits(1) {
            Some(has_bits) => {
                if has_bits {
                    let mut new_bytes: Vec<u8> =
                        self.bytes[self.byte_offset..self.bytes.len()].to_vec();
                    new_bytes.extend(bytes);
                    self.bytes = new_bytes;
                    self.byte_offset = 0;
                } else {
                    self.bytes = bytes.to_vec();
                    self.byte_offset = 0;
                    self.bit_offset = 0;
                }
            }
            None => {},
        }
    }
}