//! Exact finite decimal input. Retain author spelling separately from arithmetic.
use num_bigint::BigUint;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub const MAX_NUMBER_BYTES: usize = 1024;
pub const MAX_DECIMAL_EXPONENT: i32 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NumberError {
    #[error("chart number must be a finite decimal with optional exponent")]
    Invalid,
    #[error("chart number exceeds its lexical or decimal exponent budget")]
    Limit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DecimalNumber {
    lexical: String,
    pub(crate) coefficient: BigUint,
    pub(crate) exponent: i32,
    pub(crate) coefficient_digits: usize,
    negative: bool,
}

impl DecimalNumber {
    pub fn lexical(&self) -> &str {
        &self.lexical
    }
    pub fn is_zero(&self) -> bool {
        self.coefficient == BigUint::from(0u8)
    }
    /// Keeps a negative zero's sign; consumers decide its semantic significance.
    pub fn is_sign_negative(&self) -> bool {
        self.negative
    }
}

impl TryFrom<String> for DecimalNumber {
    type Error = NumberError;
    fn try_from(lexical: String) -> Result<Self, Self::Error> {
        if lexical.len() > MAX_NUMBER_BYTES {
            return Err(NumberError::Limit);
        }
        let text = lexical.trim_matches([' ', '\t', '\r', '\n']);
        let bytes = text.as_bytes();
        let mut offset = 0;
        let negative = bytes.first() == Some(&b'-');
        if matches!(bytes.first(), Some(b'+' | b'-')) {
            offset += 1;
        }
        let mut digits = Vec::with_capacity(bytes.len());
        while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
            digits.push(bytes[offset]);
            offset += 1;
        }
        let whole_digits = digits.len();
        if bytes.get(offset) == Some(&b'.') {
            offset += 1;
            while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
                digits.push(bytes[offset]);
                offset += 1;
            }
        }
        if digits.is_empty() {
            return Err(NumberError::Invalid);
        }
        let fraction_digits = (digits.len() - whole_digits) as i32;
        let mut exponent = 0i32;
        if matches!(bytes.get(offset), Some(b'e' | b'E')) {
            offset += 1;
            let exponent_negative = bytes.get(offset) == Some(&b'-');
            if matches!(bytes.get(offset), Some(b'+' | b'-')) {
                offset += 1;
            }
            let start = offset;
            while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
                exponent = exponent
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(i32::from(bytes[offset] - b'0')))
                    .filter(|v| *v <= MAX_DECIMAL_EXPONENT)
                    .ok_or(NumberError::Limit)?;
                offset += 1;
            }
            if start == offset {
                return Err(NumberError::Invalid);
            }
            if exponent_negative {
                exponent = -exponent;
            }
        }
        if offset != bytes.len() {
            return Err(NumberError::Invalid);
        }
        exponent -= fraction_digits;
        let first = digits.iter().position(|d| *d != b'0');
        let (coefficient, coefficient_digits) = if let Some(first) = first {
            let mut last = digits.len();
            while digits[last - 1] == b'0' {
                last -= 1;
                exponent += 1;
            }
            (
                BigUint::parse_bytes(&digits[first..last], 10).ok_or(NumberError::Invalid)?,
                last - first,
            )
        } else {
            exponent = 0;
            (BigUint::from(0u8), 1)
        };
        if exponent.abs() > MAX_DECIMAL_EXPONENT {
            return Err(NumberError::Limit);
        }
        Ok(Self {
            lexical,
            coefficient,
            exponent,
            coefficient_digits,
            negative,
        })
    }
}

impl From<DecimalNumber> for String {
    fn from(number: DecimalNumber) -> String {
        number.lexical
    }
}
impl JsonSchema for DecimalNumber {
    fn schema_name() -> Cow<'static, str> {
        "ChartDecimalNumber".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type":"string", "minLength":1, "maxLength":MAX_NUMBER_BYTES,
            "pattern":"^[ \\t\\r\\n]*[+-]?([0-9]+(\\.[0-9]*)?|\\.[0-9]+)([eE][+-]?[0-9]+)?[ \\t\\r\\n]*$",
            "description":"Exact finite decimal spelling, never a JSON floating point number. Semantic validation limits normalized decimal exponent to -4096..4096 and lexical UTF-8 bytes to 1024. Null, blanks, error tokens and infinities are not numeric weights."
        })
    }
}
