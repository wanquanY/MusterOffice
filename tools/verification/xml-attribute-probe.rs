//! Diagnostic-only Native/WASM probe. Binary requests, checked indices, no raw
//! memory pointers. This is not a public document operation or product host.
use mo_xml::{AttributeEdit, AttributeRewriteLimits, ExpandedName, XmlError, rewrite_attributes};
use std::cell::{Cell, RefCell};
const MAX: usize = 4 * 1024 * 1024;
struct Input<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Input<'a> {
    fn word(&mut self) -> Option<usize> {
        let b = self.bytes.get(self.at..self.at.checked_add(4)?)?;
        self.at += 4;
        Some(u32::from_le_bytes(b.try_into().ok()?) as usize)
    }
    fn data(&mut self) -> Option<&'a [u8]> {
        let n = self.word()?;
        let end = self.at.checked_add(n)?;
        let v = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(v)
    }
    fn string(&mut self) -> Option<String> {
        Some(std::str::from_utf8(self.data()?).ok()?.to_owned())
    }
    fn optional(&mut self) -> Option<Option<String>> {
        let present = self.word()?;
        if present == 0 {
            Some(None)
        } else if present == 1 {
            Some(Some(self.string()?))
        } else {
            None
        }
    }
    fn name(&mut self) -> Option<ExpandedName> {
        Some(ExpandedName {
            namespace: self.string()?,
            local: self.string()?,
        })
    }
}
fn request(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() > MAX {
        return None;
    }
    let mut r = Input { bytes, at: 0 };
    let stop = r.word()?;
    let mut limits = AttributeRewriteLimits::default();
    limits.xml.max_bytes = r.word()?;
    limits.max_edits = r.word()?;
    limits.max_edit_bytes = r.word()?;
    limits.xml.max_attributes = r.word()?;
    limits.xml.max_attribute_bytes = r.word()?;
    let source = r.data()?;
    let count = r.word()?;
    if count > 10_000 {
        return None;
    }
    let mut edits = Vec::new();
    for _ in 0..count {
        edits.push(AttributeEdit {
            element_ordinal: r.word()?,
            expected_element: r.name()?,
            attribute: r.name()?,
            expected_value: r.optional()?,
            value: r.optional()?,
            insertion_name: r.optional()?,
        });
    }
    if r.at != bytes.len() {
        return None;
    }
    let calls = Cell::new(0usize);
    let check = || {
        calls.set(calls.get() + 1);
        stop != 0 && calls.get() >= stop
    };
    let result = rewrite_attributes(source, &edits, limits, &check);
    let (code, body) = match result {
        Ok(v) => (0u32, v),
        Err(e) => {
            let code = match &e {
                XmlError::Cancelled => 1,
                XmlError::Limit(_) => 2,
                XmlError::EditConflict(_) => 3,
                _ => 4,
            };
            (code, e.to_string().into_bytes())
        }
    };
    let mut out = code.to_le_bytes().to_vec();
    out.extend_from_slice(&(calls.get() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Some(out)
}
thread_local! {static STATE:RefCell<(Vec<u8>,Vec<u8>)>=const {RefCell::new((Vec::new(),Vec::new()))};}
#[unsafe(no_mangle)]
pub extern "C" fn reset(n: u32) -> u32 {
    if n as usize > MAX {
        return 0;
    }
    STATE.with(|s| {
        *s.borrow_mut() = (vec![0; n as usize], Vec::new());
    });
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn put(i: u32, v: u32) -> u32 {
    if v > 255 {
        return 0;
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(at) = s.0.get_mut(i as usize) {
            *at = v as u8;
            1
        } else {
            0
        }
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn execute() -> u32 {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.1 = request(&s.0).unwrap_or_else(|| 5u32.to_le_bytes().to_vec());
        s.1.len() as u32
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn get(i: u32) -> u32 {
    STATE.with(|s| s.borrow().1.get(i as usize).map_or(256, |v| u32::from(*v)))
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::io::{Read, Write};
    let mut all = Vec::new();
    std::io::stdin()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut all)
        .unwrap();
    let mut r = Input { bytes: &all, at: 0 };
    let n = r.word().unwrap();
    assert!(n < 100_000);
    let mut out = std::io::stdout().lock();
    for _ in 0..n {
        let result = request(r.data().unwrap()).unwrap();
        out.write_all(&(result.len() as u32).to_le_bytes()).unwrap();
        out.write_all(&result).unwrap();
    }
    assert_eq!(r.at, all.len());
}
