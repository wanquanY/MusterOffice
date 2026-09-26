//! Fixed Unicode 18 data; neither the library nor the host supplies properties.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use unicode_bidi::{BidiDataSource, data_source::BidiMatchedOpeningBracket};
const DATA: &[u8] = include_bytes!("../../data/bidi.bin");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum BidiClass {
    L,
    R,
    Al,
    En,
    Es,
    Et,
    An,
    Cs,
    Nsm,
    Bn,
    B,
    S,
    Ws,
    On,
    Lre,
    Lro,
    Rle,
    Rlo,
    Pdf,
    Lri,
    Rli,
    Fsi,
    Pdi,
}
const CLASSES: [BidiClass; 23] = [
    BidiClass::L,
    BidiClass::R,
    BidiClass::Al,
    BidiClass::En,
    BidiClass::Es,
    BidiClass::Et,
    BidiClass::An,
    BidiClass::Cs,
    BidiClass::Nsm,
    BidiClass::Bn,
    BidiClass::B,
    BidiClass::S,
    BidiClass::Ws,
    BidiClass::On,
    BidiClass::Lre,
    BidiClass::Lro,
    BidiClass::Rle,
    BidiClass::Rlo,
    BidiClass::Pdf,
    BidiClass::Lri,
    BidiClass::Rli,
    BidiClass::Fsi,
    BidiClass::Pdi,
];
impl BidiClass {
    pub(super) fn removed(self) -> bool {
        matches!(
            self,
            Self::Lre | Self::Rle | Self::Lro | Self::Rlo | Self::Pdf | Self::Bn
        )
    }
    pub(super) fn whitespace(self) -> bool {
        matches!(
            self,
            Self::Ws | Self::Lri | Self::Rli | Self::Fsi | Self::Pdi
        )
    }
    fn library(self) -> unicode_bidi::BidiClass {
        use unicode_bidi::BidiClass as C;
        match self {
            Self::L => C::L,
            Self::R => C::R,
            Self::Al => C::AL,
            Self::En => C::EN,
            Self::Es => C::ES,
            Self::Et => C::ET,
            Self::An => C::AN,
            Self::Cs => C::CS,
            Self::Nsm => C::NSM,
            Self::Bn => C::BN,
            Self::B => C::B,
            Self::S => C::S,
            Self::Ws => C::WS,
            Self::On => C::ON,
            Self::Lre => C::LRE,
            Self::Lro => C::LRO,
            Self::Rle => C::RLE,
            Self::Rlo => C::RLO,
            Self::Pdf => C::PDF,
            Self::Lri => C::LRI,
            Self::Rli => C::RLI,
            Self::Fsi => C::FSI,
            Self::Pdi => C::PDI,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BidiBracket {
    pub paired: u32,
    pub normalized_opening: u32,
    pub is_open: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BidiProperties {
    pub class: BidiClass,
    pub bracket: Option<BidiBracket>,
    pub mirrored: bool,
    /// Informative mapping, not a direction-independent character replacement.
    pub mirroring_glyph: Option<u32>,
}
fn u32_at(at: usize) -> u32 {
    u32::from_le_bytes(DATA[at..at + 4].try_into().expect("generated bidi table"))
}
fn flags(cp: u32) -> u8 {
    let (mut lo, mut hi) = (0, u32_at(8) as usize);
    while lo < hi {
        let mid = (lo + hi) / 2;
        let at = 20 + mid * 9;
        if cp < u32_at(at) {
            hi = mid;
        } else if cp > u32_at(at + 4) {
            lo = mid + 1;
        } else {
            return DATA[at + 8];
        }
    }
    0
}
pub fn bidi_class(c: char) -> BidiClass {
    CLASSES[(flags(c as u32) & 31) as usize]
}
fn find(cp: u32, start: usize, count: usize, width: usize) -> Option<usize> {
    let (mut lo, mut hi) = (0, count);
    while lo < hi {
        let mid = (lo + hi) / 2;
        let at = start + mid * width;
        match cp.cmp(&u32_at(at)) {
            std::cmp::Ordering::Less => hi = mid,
            std::cmp::Ordering::Greater => lo = mid + 1,
            std::cmp::Ordering::Equal => return Some(at),
        }
    }
    None
}
pub fn bracket(c: char) -> Option<BidiBracket> {
    let start = 20 + u32_at(8) as usize * 9;
    find(c as u32, start, u32_at(12) as usize, 13).map(|at| BidiBracket {
        paired: u32_at(at + 4),
        normalized_opening: u32_at(at + 8),
        is_open: DATA[at + 12] != 0,
    })
}
pub fn bidi_properties(c: char) -> BidiProperties {
    let bits = flags(c as u32);
    let start = 20 + u32_at(8) as usize * 9 + u32_at(12) as usize * 13;
    BidiProperties {
        class: CLASSES[(bits & 31) as usize],
        bracket: bracket(c),
        mirrored: bits & 32 != 0,
        mirroring_glyph: find(c as u32, start, u32_at(16) as usize, 8).map(|at| u32_at(at + 4)),
    }
}
pub(super) struct DataSource18;
impl BidiDataSource for DataSource18 {
    fn bidi_class(&self, c: char) -> unicode_bidi::BidiClass {
        bidi_class(c).library()
    }
    fn bidi_matched_opening_bracket(&self, c: char) -> Option<BidiMatchedOpeningBracket> {
        bracket(c).map(|b| BidiMatchedOpeningBracket {
            opening: char::from_u32(b.normalized_opening).expect("generated bracket scalar"),
            is_open: b.is_open,
        })
    }
}
