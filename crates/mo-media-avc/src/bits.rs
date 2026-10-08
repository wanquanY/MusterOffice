//! Checked RBSP prefix reader. Original implementation of H.264 clause 7 syntax.
use crate::{Result, invalid};

pub(crate) struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
    zeros: u8,
    current: u8,
    left: u8,
}
impl<'a> Bits<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            at: 0,
            zeros: 0,
            current: 0,
            left: 0,
        }
    }
    fn byte(&mut self) -> Result<u8> {
        let mut value = *self
            .bytes
            .get(self.at)
            .ok_or_else(|| invalid("truncated RBSP"))?;
        self.at += 1;
        if self.zeros == 2 {
            if value == 3 {
                value = *self
                    .bytes
                    .get(self.at)
                    .ok_or_else(|| invalid("truncated escape"))?;
                self.at += 1;
                if value > 3 {
                    return Err(invalid("invalid emulation prevention"));
                }
                self.zeros = 0;
            } else if value < 3 {
                return Err(invalid("unescaped RBSP start code"));
            }
        }
        self.zeros = if value == 0 { self.zeros + 1 } else { 0 };
        Ok(value)
    }
    pub fn u(&mut self, count: u8) -> Result<u32> {
        if count > 32 {
            return Err(invalid("bit width"));
        }
        let mut value = 0;
        for _ in 0..count {
            if self.left == 0 {
                self.current = self.byte()?;
                self.left = 8;
            }
            self.left -= 1;
            value = (value << 1) | u32::from((self.current >> self.left) & 1);
        }
        Ok(value)
    }
    pub fn flag(&mut self) -> Result<bool> {
        Ok(self.u(1)? != 0)
    }
    pub fn ue(&mut self, max: u32) -> Result<u32> {
        let mut zeros = 0;
        while !self.flag()? {
            zeros += 1;
            if zeros > 31 {
                return Err(invalid("Exp-Golomb overflow"));
            }
        }
        let value = ((1u64 << zeros) - 1) + u64::from(self.u(zeros)?);
        if value > u64::from(max) {
            return Err(invalid("Exp-Golomb range"));
        }
        Ok(value as u32)
    }
    pub fn se(&mut self) -> Result<i32> {
        let value = self.ue(u32::MAX - 1)?;
        let magnitude = value.div_ceil(2) as i32;
        Ok(if value & 1 != 0 {
            magnitude
        } else {
            -magnitude
        })
    }
    pub fn trailing(&mut self) -> Result<()> {
        if !self.flag()? {
            return Err(invalid("RBSP stop bit"));
        }
        while self.left > 0 {
            if self.flag()? {
                return Err(invalid("RBSP alignment"));
            }
        }
        if self.at != self.bytes.len() {
            return Err(invalid("trailing SPS bytes"));
        }
        Ok(())
    }
}
