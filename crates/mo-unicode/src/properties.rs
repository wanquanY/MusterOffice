use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
const DATA: &[u8] = include_bytes!("../data/properties.bin");
const HEADER: usize = 12;
const WIDTH: usize = 10;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum GraphemeBreak {
    Other,
    Cr,
    Lf,
    Control,
    Extend,
    Zwj,
    RegionalIndicator,
    Prepend,
    SpacingMark,
    L,
    V,
    T,
    Lv,
    Lvt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IndicConjunct {
    None,
    Consonant,
    Extend,
    Linker,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Properties {
    pub grapheme_break: GraphemeBreak,
    pub indic_conjunct: IndicConjunct,
    pub extended_pictographic: bool,
    pub default_ignorable: bool,
    pub variation_selector: bool,
}
fn word(offset: usize) -> u32 {
    u32::from_le_bytes(
        DATA[offset..offset + 4]
            .try_into()
            .expect("generated table bounds"),
    )
}
pub fn properties(character: char) -> Properties {
    let cp = u32::from(character);
    let mut low = 0;
    let mut high = (DATA.len() - HEADER) / WIDTH;
    let mut flags = 0;
    while low < high {
        let mid = low + (high - low) / 2;
        let offset = HEADER + mid * WIDTH;
        if cp < word(offset) {
            high = mid;
        } else if cp > word(offset + 4) {
            low = mid + 1;
        } else {
            flags = u16::from_le_bytes([DATA[offset + 8], DATA[offset + 9]]);
            break;
        }
    }
    use GraphemeBreak::*;
    Properties {
        grapheme_break: match flags & 15 {
            0 => Other,
            1 => Cr,
            2 => Lf,
            3 => Control,
            4 => Extend,
            5 => Zwj,
            6 => RegionalIndicator,
            7 => Prepend,
            8 => SpacingMark,
            9 => L,
            10 => V,
            11 => T,
            12 => Lv,
            13 => Lvt,
            _ => unreachable!("generated GCB value"),
        },
        indic_conjunct: match (flags >> 4) & 3 {
            0 => IndicConjunct::None,
            1 => IndicConjunct::Consonant,
            2 => IndicConjunct::Extend,
            3 => IndicConjunct::Linker,
            _ => unreachable!(),
        },
        extended_pictographic: flags & 64 != 0,
        default_ignorable: flags & 128 != 0,
        variation_selector: flags & 256 != 0,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_table_is_sorted_nonoverlapping_and_has_valid_values() {
        assert_eq!(&DATA[..8], b"MOUCD018");
        assert_eq!(DATA.len(), HEADER + word(8) as usize * WIDTH);
        let mut previous = None;
        for i in 0..word(8) as usize {
            let offset = HEADER + i * WIDTH;
            let start = word(offset);
            let end = word(offset + 4);
            assert!(start <= end && end <= 0x10ffff);
            assert!(previous.is_none_or(|p| p < start));
            previous = Some(end);
            let flags = u16::from_le_bytes([DATA[offset + 8], DATA[offset + 9]]);
            assert!(flags & 15 <= 13 && flags & !511 == 0 && flags != 0);
        }
    }
    #[test]
    fn selectors_match_the_explicit_font_contract() {
        for cp in 0..=0x10ffff {
            if let Some(c) = char::from_u32(cp) {
                assert_eq!(
                    properties(c).variation_selector,
                    matches!(cp,0x180b..=0x180d|0x180f|0xfe00..=0xfe0f|0xe0100..=0xe01ef)
                );
            }
        }
    }
}
