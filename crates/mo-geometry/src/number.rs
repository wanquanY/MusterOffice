//! Shared signed i128 Q32 coordinate. Canonical JSON integer string, never float.
use crate::GeometryError;
use mo_common::Emu;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
const UNIT: i128 = 1i128 << 32;
fn overflow() -> GeometryError {
    GeometryError::Numeric
}
fn rounded(n: i128, d: i128) -> Result<i128, GeometryError> {
    let q = n / d;
    let r = n % d;
    if r.abs() * 2 >= d {
        q.checked_add(n.signum()).ok_or_else(overflow)
    } else {
        Ok(q)
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Fixed(i128);
impl Fixed {
    pub const FRACTION_BITS: u32 = 32;
    pub const fn raw(self) -> i128 {
        self.0
    }
    pub const fn from_raw(raw: i128) -> Self {
        Self(raw)
    }
    pub const ZERO: Self = Self(0);
    pub fn emu(v: Emu) -> Self {
        Self(i128::from(v.get()) * UNIT)
    }
    pub fn scale(raw: i64, size: Emu, scale: u32) -> Result<Self, GeometryError> {
        if scale == 0 {
            return Err(GeometryError::Invalid("zero font position scale"));
        }
        let n = i128::from(raw) * i128::from(size.get());
        let d = i128::from(scale);
        // Divide before shifting: a large but representable result must not
        // overflow only because its unneeded rational numerator was shifted.
        let whole = (n / d).checked_mul(UNIT).ok_or_else(overflow)?;
        let fraction = rounded((n % d) * UNIT, d)?;
        whole.checked_add(fraction).map(Self).ok_or_else(overflow)
    }
    pub fn checked_add(self, other: Self) -> Result<Self, GeometryError> {
        self.0.checked_add(other.0).map(Self).ok_or_else(overflow)
    }
    pub fn checked_sub(self, other: Self) -> Result<Self, GeometryError> {
        self.0.checked_sub(other.0).map(Self).ok_or_else(overflow)
    }
    pub fn half(self) -> Result<Self, GeometryError> {
        rounded(self.0, 2).map(Self)
    }
    pub fn wire(self) -> Result<Emu, GeometryError> {
        i64::try_from(rounded(self.0, UNIT)?)
            .map(Emu::new)
            .map_err(|_| overflow())
    }
}
impl TryFrom<String> for Fixed {
    type Error = GeometryError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let v = s
            .parse::<i128>()
            .map_err(|_| GeometryError::Invalid("canonical Q32 integer"))?;
        if v.to_string() != s {
            return Err(GeometryError::Invalid("canonical Q32 integer"));
        }
        Ok(Self(v))
    }
}
impl From<Fixed> for String {
    fn from(v: Fixed) -> Self {
        v.0.to_string()
    }
}
impl JsonSchema for Fixed {
    fn schema_name() -> Cow<'static, str> {
        "FixedQ32".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string","pattern":"^(0|-?[1-9][0-9]{0,38})$","not":{"pattern":"[^0-9-]"},"x-integer-minimum":"-170141183460469231731687303715884105728","x-integer-maximum":"170141183460469231731687303715884105727","description":"Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required."})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_ties_large_values_and_range_are_explicit() {
        for sign in [-1, 1] {
            assert_eq!(
                Fixed::scale(sign, Emu::new(1), 2)
                    .unwrap()
                    .wire()
                    .unwrap()
                    .get(),
                sign
            );
            assert_eq!(
                Fixed::scale(sign * 3, Emu::new(1), 2)
                    .unwrap()
                    .wire()
                    .unwrap()
                    .get(),
                sign * 2
            );
        }
        assert_eq!(
            Fixed::scale(i64::MAX, Emu::new(64), 64)
                .unwrap()
                .wire()
                .unwrap()
                .get(),
            i64::MAX
        );
        assert!(
            Fixed::scale(i64::MAX, Emu::new(65), 64)
                .unwrap()
                .wire()
                .is_err()
        );
        assert!(Fixed::scale(i64::MAX, Emu::new(i64::MAX), 1).is_err());
        assert!(Fixed::scale(1, Emu::new(1), 0).is_err());
    }
    #[test]
    fn design_prefix_quantization_is_not_per_glyph_wire_rounding() {
        let size = Emu::new(1);
        assert_eq!(Fixed::scale(1, size, 3).unwrap().wire().unwrap().get(), 0);
        assert_eq!(
            Fixed::scale(300, size, 3).unwrap().wire().unwrap().get(),
            100
        );
    }
}
