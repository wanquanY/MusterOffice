//! Source format-code evaluation on exact decimals and ratios. No implicit
//! locale, binary floats, cell-width guesses or font-dependent space folding.
mod parse;
mod render;
mod types;
use crate::DecimalNumber;
use num_bigint::BigUint;
use std::collections::BTreeMap;
pub use types::*;

pub struct Formatter<'a> {
    symbols: &'a NumberSymbols,
    limits: FormatLimits,
    check: &'a dyn Fn() -> bool,
    formats: BTreeMap<String, Result<parse::Format, NumberFormatIssue>>,
    code_bytes: usize,
    numeric_digits: usize,
    output_bytes: usize,
}
impl<'a> Formatter<'a> {
    pub fn new(
        symbols: &'a NumberSymbols,
        limits: FormatLimits,
        check: &'a dyn Fn() -> bool,
    ) -> Result<Self, FormatError> {
        if symbols.decimal_separator == symbols.group_separator
            || [&symbols.decimal_separator, &symbols.group_separator]
                .iter()
                .any(|s| {
                    s.is_empty()
                        || s.len() > 16
                        || s.chars().any(|c| c.is_control() || c.is_ascii_digit())
                })
        {
            return Err(FormatError::Symbols);
        }
        Ok(Self {
            symbols,
            limits,
            check,
            formats: BTreeMap::new(),
            code_bytes: 0,
            numeric_digits: 0,
            output_bytes: 0,
        })
    }
    pub fn decimal(
        &mut self,
        value: &DecimalNumber,
        code: &str,
    ) -> Result<NumberDisplay, FormatError> {
        self.poll()?;
        self.digits(value.coefficient_digits + value.exponent.unsigned_abs() as usize + 1)?;
        let power = BigUint::from(10u8).pow(value.exponent.unsigned_abs());
        let (n, d) = if value.exponent < 0 {
            (value.coefficient.clone(), power)
        } else {
            (&value.coefficient * power, BigUint::from(1u8))
        };
        self.evaluate(n, d, value.is_sign_negative(), code)
    }
    pub fn ratio(
        &mut self,
        numerator: &str,
        denominator: &str,
        code: &str,
    ) -> Result<NumberDisplay, FormatError> {
        self.poll()?;
        self.digits(numerator.len().saturating_add(denominator.len()))?;
        if [numerator, denominator]
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(FormatError::Ratio);
        }
        let n = BigUint::parse_bytes(numerator.as_bytes(), 10).ok_or(FormatError::Ratio)?;
        let d = BigUint::parse_bytes(denominator.as_bytes(), 10).ok_or(FormatError::Ratio)?;
        if d == BigUint::from(0u8) {
            return Err(FormatError::Ratio);
        }
        self.evaluate(n, d, false, code)
    }
    fn evaluate(
        &mut self,
        n: BigUint,
        d: BigUint,
        negative: bool,
        code: &str,
    ) -> Result<NumberDisplay, FormatError> {
        self.poll()?;
        if !self.formats.contains_key(code) {
            if code.len() > self.limits.max_code_bytes
                || self.formats.len() >= self.limits.max_formats
            {
                return Err(FormatError::Limit("format codes"));
            }
            charge(
                &mut self.code_bytes,
                code.len(),
                self.limits.max_total_code_bytes,
                "total format code bytes",
            )?;
            let parsed = parse::compile(code, self.limits)?;
            self.formats.insert(code.to_owned(), parsed);
        }
        // Evaluation borrows the compiled code. A local work record is committed
        // after either result so unsupported inputs cannot reset work budgets.
        let format = self
            .formats
            .get(code)
            .expect("inserted format")
            .as_ref()
            .map_err(|e| FormatError::Unresolved(e.clone()))?;
        let mut work = render::Work {
            limits: self.limits,
            digits: self.numeric_digits,
            bytes: self.output_bytes,
            check: self.check,
        };
        let result = render::evaluate(format, n, d, negative, self.symbols, &mut work);
        self.numeric_digits = work.digits;
        self.output_bytes = work.bytes;
        result
    }
    fn digits(&mut self, n: usize) -> Result<(), FormatError> {
        if n > self.limits.max_numeric_digits {
            return Err(FormatError::Limit("numeric digits"));
        }
        charge(
            &mut self.numeric_digits,
            n,
            self.limits.max_total_numeric_digits,
            "total numeric digits",
        )
    }
    fn poll(&self) -> Result<(), FormatError> {
        if (self.check)() {
            Err(FormatError::Cancelled)
        } else {
            Ok(())
        }
    }
}
fn charge(
    total: &mut usize,
    n: usize,
    limit: usize,
    name: &'static str,
) -> Result<(), FormatError> {
    *total = total
        .checked_add(n)
        .filter(|v| *v <= limit)
        .ok_or(FormatError::Limit(name))?;
    Ok(())
}
