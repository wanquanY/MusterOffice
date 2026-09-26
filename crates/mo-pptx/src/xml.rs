use crate::{PptxError, value};

/// Small bounded emitter. Dynamic values only enter through attribute/text escaping.
pub(crate) struct Xml {
    bytes: String,
    max: usize,
}
impl Xml {
    pub fn new(max: usize) -> Result<Self, PptxError> {
        let mut result = Self {
            bytes: String::new(),
            max,
        };
        result.raw("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>")?;
        Ok(result)
    }
    pub fn raw(&mut self, trusted: &str) -> Result<(), PptxError> {
        if trusted.len() > self.max.saturating_sub(self.bytes.len()) {
            return Err(PptxError::Limit("XML output bytes"));
        }
        self.bytes.push_str(trusted);
        Ok(())
    }
    pub fn attr(&mut self, name: &str, value: impl ToString) -> Result<(), PptxError> {
        self.raw(" ")?;
        self.raw(name)?;
        self.raw("=\"")?;
        self.escaped(&value.to_string(), true)?;
        self.raw("\"")
    }
    pub fn text(&mut self, value: &str) -> Result<(), PptxError> {
        self.escaped(value, false)
    }
    fn escaped(&mut self, text: &str, attribute: bool) -> Result<(), PptxError> {
        for character in text.chars() {
            match character {
                '&' => self.raw("&amp;")?,
                '<' => self.raw("&lt;")?,
                '>' => self.raw("&gt;")?,
                '"' if attribute => self.raw("&quot;")?,
                '\r' => self.raw("&#xD;")?,
                '\n' if attribute => self.raw("&#xA;")?,
                '\t' if attribute => self.raw("&#x9;")?,
                c if matches!(c as u32, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF) =>
                {
                    let mut bytes = [0; 4];
                    self.raw(c.encode_utf8(&mut bytes))?;
                }
                _ => return Err(value("XML text", "invalid XML 1.0 character")),
            }
        }
        Ok(())
    }
    pub fn finish(self) -> Vec<u8> {
        self.bytes.into_bytes()
    }
}
