//! Owned, bounded component inputs. Handles never recycle within a backend.
use mo_text::TextError;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Default)]
pub(crate) struct Fonts {
    values: BTreeMap<u32, Arc<[u8]>>,
    bytes: usize,
    next: u32,
}
impl Fonts {
    pub fn insert(&mut self, font: &[u8]) -> Result<u32, TextError> {
        let bytes = self
            .bytes
            .checked_add(font.len())
            .ok_or(TextError::Limit("resident font bytes"))?;
        if font.is_empty() || bytes > 128 * 1024 * 1024 || self.values.len() >= 32 {
            return Err(TextError::Limit("resident fonts"));
        }
        let id = self
            .next
            .checked_add(1)
            .ok_or(TextError::Limit("resident font handles"))?;
        self.values.insert(id, Arc::from(font));
        self.bytes = bytes;
        self.next = id;
        Ok(id)
    }
    pub fn get(&self, id: u32) -> Result<Arc<[u8]>, TextError> {
        self.values
            .get(&id)
            .cloned()
            .ok_or(TextError::BackendInvalid("unknown resident font"))
    }
    pub fn remove(&mut self, id: u32) -> Result<(), TextError> {
        let font = self
            .values
            .remove(&id)
            .ok_or(TextError::BackendInvalid("unknown resident font"))?;
        self.bytes -= font.len();
        Ok(())
    }
    pub fn clear(&mut self) {
        self.values.clear();
        self.bytes = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn released_handles_do_not_alias_later_fonts() {
        let mut fonts = Fonts::default();
        let a = fonts.insert(&[1, 2, 3]).unwrap();
        fonts.remove(a).unwrap();
        let b = fonts.insert(&[4, 5, 6]).unwrap();
        assert_ne!(a, b);
        assert!(fonts.get(a).is_err());
        assert_eq!(&*fonts.get(b).unwrap(), &[4, 5, 6]);
        fonts.clear();
        assert!(fonts.get(b).is_err());
        assert_eq!(fonts.bytes, 0);
    }
    #[test]
    fn residency_is_bounded_and_empty_fonts_rejected() {
        let mut fonts = Fonts::default();
        assert!(fonts.insert(&[]).is_err());
        for _ in 0..32 {
            fonts.insert(&[1]).unwrap();
        }
        assert!(fonts.insert(&[1]).is_err());
        fonts.remove(1).unwrap();
        assert!(fonts.insert(&[2]).is_ok());
    }
}
