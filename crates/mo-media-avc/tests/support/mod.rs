#![allow(dead_code)]
pub struct Bits(Vec<bool>);
impl Bits {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn u(&mut self, value: u32, count: u8) {
        for bit in (0..count).rev() {
            self.0.push(value & (1 << bit) != 0);
        }
    }
    pub fn flag(&mut self, value: bool) {
        self.u(u32::from(value), 1);
    }
    pub fn ue(&mut self, value: u32) {
        let n = value + 1;
        let bits = (32 - n.leading_zeros()) as u8;
        self.u(0, bits - 1);
        self.u(n, bits);
    }
    pub fn nal(mut self, header: u8) -> Vec<u8> {
        self.flag(true);
        while !self.0.len().is_multiple_of(8) {
            self.flag(false);
        }
        let mut result = vec![header];
        let mut zeros = 0;
        for chunk in self.0.chunks(8) {
            let byte = chunk.iter().fold(0, |value, b| value * 2 + u8::from(*b));
            if zeros == 2 && byte <= 3 {
                result.push(3);
                zeros = 0;
            }
            result.push(byte);
            zeros = if byte == 0 { zeros + 1 } else { 0 };
        }
        result
    }
}
pub fn sps(id: u32, chroma: u32, location: Option<(u32, u32)>, colors: bool) -> Vec<u8> {
    let mut b = Bits::new();
    b.u(100, 8);
    b.u(0, 8);
    b.u(31, 8);
    b.ue(id);
    b.ue(chroma);
    if chroma == 3 {
        b.flag(false);
    }
    b.ue(0);
    b.ue(0);
    b.flag(false);
    b.flag(false);
    b.ue(0);
    b.ue(0);
    b.ue(0); // frame bits, POC type/bits
    b.ue(2);
    b.flag(false);
    b.ue(7);
    b.ue(5); // 128x96
    b.flag(true);
    b.flag(true);
    b.flag(false);
    b.flag(true); // VUI
    b.flag(false);
    b.flag(false);
    b.flag(colors);
    if colors {
        b.u(5, 3);
        b.flag(true);
        b.flag(true);
        b.u(9, 8);
        b.u(16, 8);
        b.u(9, 8);
    }
    b.flag(location.is_some());
    if let Some((top, bottom)) = location {
        b.ue(top);
        b.ue(bottom);
    }
    b.flag(true);
    b.u(1001, 32);
    b.u(60000, 32);
    b.flag(true); // includes RBSP escapes
    b.flag(false);
    b.flag(false);
    b.flag(false); // HRD, picture structure
    b.flag(true);
    b.flag(true);
    b.ue(2);
    b.ue(1);
    b.ue(16);
    b.ue(16);
    b.ue(2);
    b.ue(2);
    b.nal(0x67)
}
pub fn pps(id: u32, sps: u32) -> Vec<u8> {
    let mut b = Bits::new();
    b.ue(id);
    b.ue(sps);
    b.flag(false);
    b.flag(false);
    b.ue(0);
    b.ue(0);
    b.ue(0);
    b.flag(false);
    b.u(0, 2);
    b.ue(0);
    b.ue(0);
    b.ue(0);
    b.flag(true);
    b.flag(false);
    b.flag(false);
    b.nal(0x68)
}
pub fn slice(pps: u32, idr: bool, frame: u32) -> Vec<u8> {
    let mut b = Bits::new();
    b.ue(0);
    b.ue(2);
    b.ue(pps);
    b.u(frame, 4);
    if idr {
        b.ue(0);
    }
    b.u(frame, 4);
    // Header prefix only: this test doesn't claim a decodable picture.
    b.nal(if idr { 0x65 } else { 0x41 })
}
pub fn config(sps: &[Vec<u8>], pps: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![1, 100, 0, 31, 0xff, 0xe0 | sps.len() as u8];
    for nal in sps {
        bytes.extend_from_slice(&(nal.len() as u16).to_be_bytes());
        bytes.extend_from_slice(nal);
    }
    bytes.push(pps.len() as u8);
    for nal in pps {
        bytes.extend_from_slice(&(nal.len() as u16).to_be_bytes());
        bytes.extend_from_slice(nal);
    }
    bytes
}
pub fn packet(nals: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![];
    for nal in nals {
        bytes.extend_from_slice(&(nal.len() as u32).to_be_bytes());
        bytes.extend_from_slice(nal);
    }
    bytes
}
