//! Unicode 18 Script and Script_Extensions, without locale/OS inference.
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
const DATA: &[u8] = include_bytes!("../data/scripts.bin");
fn u16_at(at: usize) -> u16 {
    u16::from_le_bytes(DATA[at..at + 2].try_into().unwrap())
}
fn u32_at(at: usize) -> usize {
    u32::from_le_bytes(DATA[at..at + 4].try_into().unwrap()) as usize
}
/// Canonical UCD short alias; indexes are private and not wire identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Script(u16);
impl Script {
    pub fn from_tag(tag: &str) -> Option<Self> {
        let mut low = 0;
        let mut high = u32_at(8);
        while low < high {
            let middle = (low + high) / 2;
            let script = Self(middle as u16);
            match script.tag().cmp(tag) {
                std::cmp::Ordering::Equal => return Some(script),
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
            }
        }
        None
    }
    pub fn tag(self) -> &'static str {
        let at = 24 + usize::from(self.0) * 4;
        std::str::from_utf8(&DATA[at..at + 4]).unwrap()
    }
    pub fn common() -> Self {
        Self::from_tag("Zyyy").unwrap()
    }
    pub fn inherited() -> Self {
        Self::from_tag("Zinh").unwrap()
    }
    pub fn unknown() -> Self {
        Self::from_tag("Zzzz").unwrap()
    }
    pub fn is_contextual(self) -> bool {
        self == Self::common() || self == Self::inherited()
    }
}
impl Serialize for Script {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.tag())
    }
}
impl<'de> Deserialize<'de> for Script {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let tag = String::deserialize(d)?;
        Self::from_tag(&tag)
            .ok_or_else(|| serde::de::Error::custom("canonical Unicode 18 script tag required"))
    }
}
impl JsonSchema for Script {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Script".into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = String::json_schema(generator);
        schema.insert(
            "enum".into(),
            (0..u32_at(8))
                .map(|i| Self(i as u16).tag())
                .collect::<Vec<_>>()
                .into(),
        );
        schema
    }
}
/// Fixed-size set operations used by the itemizer; never allocates per scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptSet([u64; 4]);
impl ScriptSet {
    pub fn singleton(script: Script) -> Self {
        let mut v = [0; 4];
        v[usize::from(script.0) / 64] = 1 << (script.0 % 64);
        Self(v)
    }
    pub fn contains(self, script: Script) -> bool {
        self.0[usize::from(script.0) / 64] & (1 << (script.0 % 64)) != 0
    }
    pub fn intersection(self, other: Self) -> Self {
        Self(std::array::from_fn(|i| self.0[i] & other.0[i]))
    }
    pub fn is_empty(self) -> bool {
        self.0 == [0; 4]
    }
    pub fn iter(self) -> impl Iterator<Item = Script> {
        (0..u32_at(8))
            .map(|i| Script(i as u16))
            .filter(move |s| self.contains(*s))
    }
    pub fn contextual(self) -> bool {
        self == Self::singleton(Script::common()) || self == Self::singleton(Script::inherited())
    }
}
fn find(cp: u32, start: usize, count: usize) -> Option<u16> {
    let mut low = 0;
    let mut high = count;
    while low < high {
        let mid = (low + high) / 2;
        let at = start + mid * 10;
        if cp < u32_at(at) as u32 {
            high = mid;
        } else if cp > u32_at(at + 4) as u32 {
            low = mid + 1;
        } else {
            return Some(u16_at(at + 8));
        }
    }
    None
}
pub fn script(c: char) -> Script {
    find(c as u32, 24 + u32_at(8) * 4, u32_at(12))
        .map(Script)
        .unwrap_or_else(Script::unknown)
}
pub fn script_extensions(c: char) -> ScriptSet {
    let at = 24 + u32_at(8) * 4 + u32_at(12) * 10;
    match find(c as u32, at, u32_at(16)) {
        None => ScriptSet::singleton(script(c)),
        Some(index) => {
            let at = at + u32_at(16) * 10 + usize::from(index) * 32;
            ScriptSet(std::array::from_fn(|i| {
                u64::from_le_bytes(DATA[at + i * 8..at + i * 8 + 8].try_into().unwrap())
            }))
        }
    }
}
