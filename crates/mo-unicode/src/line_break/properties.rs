use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
const DATA: &[u8] = include_bytes!("../../data/line-break.bin");
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum LineBreakClass {
    Xx,
    Ai,
    Ak,
    Al,
    Ap,
    As,
    B2,
    Ba,
    Bb,
    Bk,
    Cb,
    Cj,
    Cl,
    Cm,
    Cp,
    Cr,
    Eb,
    Em,
    Ex,
    Gl,
    H2,
    H3,
    Hh,
    Hl,
    Hy,
    Id,
    In,
    Is,
    Jl,
    Jt,
    Jv,
    Lf,
    Nl,
    Ns,
    Nu,
    Op,
    Po,
    Pr,
    Qu,
    Ri,
    Sa,
    Sg,
    Sp,
    Sy,
    Vf,
    Vi,
    Wj,
    Zw,
    Zwj,
}
const CLASSES: &[LineBreakClass] = {
    use LineBreakClass::*;
    &[
        Xx, Ai, Ak, Al, Ap, As, B2, Ba, Bb, Bk, Cb, Cj, Cl, Cm, Cp, Cr, Eb, Em, Ex, Gl, H2, H3, Hh,
        Hl, Hy, Id, In, Is, Jl, Jt, Jv, Lf, Nl, Ns, Nu, Op, Po, Pr, Qu, Ri, Sa, Sg, Sp, Sy, Vf, Vi,
        Wj, Zw, Zwj,
    ]
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakProperties {
    pub class: LineBreakClass,
    /// General_Category is Mn or Mc (not all marks).
    pub combining_mark: bool,
    pub initial_punctuation: bool,
    pub final_punctuation: bool,
    pub unassigned: bool,
    /// East_Asian_Width is F, W or H; not a measured display width.
    pub east_asian: bool,
}
fn u32_at(at: usize) -> u32 {
    u32::from_le_bytes(DATA[at..at + 4].try_into().unwrap())
}
pub fn line_break_properties(c: char) -> LineBreakProperties {
    let cp = c as u32;
    let (mut lo, mut hi) = (0, u32_at(8) as usize);
    let mut packed = 512;
    while lo < hi {
        let mid = (lo + hi) / 2;
        let at = 12 + mid * 10;
        if cp < u32_at(at) {
            hi = mid;
        } else if cp > u32_at(at + 4) {
            lo = mid + 1;
        } else {
            packed = u16::from_le_bytes(DATA[at + 8..at + 10].try_into().unwrap());
            break;
        }
    }
    LineBreakProperties {
        class: CLASSES[(packed & 63) as usize],
        combining_mark: packed & 64 != 0,
        initial_punctuation: packed & 128 != 0,
        final_punctuation: packed & 256 != 0,
        unassigned: packed & 512 != 0,
        east_asian: packed & 1024 != 0,
    }
}
