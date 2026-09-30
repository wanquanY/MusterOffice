use super::{parse::*, *};

pub(super) struct Work<'a> {
    pub limits: FormatLimits,
    pub digits: usize,
    pub bytes: usize,
    pub check: &'a dyn Fn() -> bool,
}
impl Work<'_> {
    fn poll(&self) -> Result<(), FormatError> {
        if (self.check)() {
            Err(FormatError::Cancelled)
        } else {
            Ok(())
        }
    }
    fn digits(&mut self, n: usize) -> Result<(), FormatError> {
        self.poll()?;
        if n > self.limits.max_numeric_digits {
            return Err(FormatError::Limit("expanded numeric digits"));
        }
        charge(
            &mut self.digits,
            n,
            self.limits.max_total_numeric_digits,
            "total numeric digits",
        )
    }
    fn output(&mut self, n: usize) -> Result<(), FormatError> {
        self.poll()?;
        charge(
            &mut self.bytes,
            n,
            self.limits.max_output_bytes,
            "output bytes",
        )
    }
    fn power(&mut self, power: u32) -> Result<BigUint, FormatError> {
        self.digits(power as usize + 1)?;
        Ok(BigUint::from(10u8).pow(power))
    }
    fn scale(
        &mut self,
        n: BigUint,
        d: BigUint,
        power: i32,
    ) -> Result<(BigUint, BigUint), FormatError> {
        self.digits(
            n.to_str_radix(10).len() + d.to_str_radix(10).len() + power.unsigned_abs() as usize,
        )?;
        if power >= 0 {
            Ok((n * self.power(power as u32)?, d))
        } else {
            Ok((n, d * self.power(power.unsigned_abs())?))
        }
    }
    fn round(
        &mut self,
        n: &BigUint,
        d: &BigUint,
        places: usize,
    ) -> Result<(String, bool), FormatError> {
        let (n, d) = self.scale(n.clone(), d.clone(), places as i32)?;
        let mut q = &n / &d;
        let r = &n % &d;
        let changed = r != BigUint::from(0u8);
        if &r * 2u8 >= d {
            q += 1u8;
        }
        Ok((q.to_str_radix(10), changed))
    }
}
struct Output<'a, 'b> {
    fragments: Vec<NumberFragment>,
    work: &'a mut Work<'b>,
}
impl Output<'_, '_> {
    fn text(&mut self, s: &str) -> Result<(), FormatError> {
        if s.is_empty() {
            return Ok(());
        }
        self.work.output(s.len())?;
        if let Some(NumberFragment::Text { value }) = self.fragments.last_mut() {
            value.push_str(s);
        } else {
            self.work.output(32)?;
            self.fragments
                .push(NumberFragment::Text { value: s.into() });
        }
        Ok(())
    }
    fn character(&mut self, c: char) -> Result<(), FormatError> {
        self.text(c.encode_utf8(&mut [0; 4]))
    }
    fn reserve(&mut self, glyph: char) -> Result<(), FormatError> {
        self.work.output(32 + glyph.len_utf8())?;
        self.fragments.push(NumberFragment::Reserve { glyph });
        Ok(())
    }
    fn token(&mut self, t: &Token) -> Result<(), FormatError> {
        match t {
            Token::Literal(c) => self.character(*c),
            Token::Percent => self.text("%"),
            Token::Reserve(c) => self.reserve(*c),
            Token::Fill(glyph) => {
                self.work.output(32 + glyph.len_utf8())?;
                self.fragments.push(NumberFragment::Fill { glyph: *glyph });
                Ok(())
            }
            _ => unreachable!("numeric token passed to literal writer"),
        }
    }
    fn tokens(&mut self, t: &[Token]) -> Result<(), FormatError> {
        for t in t {
            self.token(t)?;
        }
        Ok(())
    }
}
pub(super) fn evaluate(
    format: &Format,
    n: BigUint,
    d: BigUint,
    negative: bool,
    symbols: &NumberSymbols,
    work: &mut Work<'_>,
) -> Result<NumberDisplay, FormatError> {
    work.poll()?;
    let zero = n == BigUint::from(0u8);
    let negative = negative && !zero;
    let section = if zero && format.sections.len() >= 3 {
        2
    } else if negative && format.sections.len() >= 2 {
        1
    } else {
        0
    };
    let selected = &format.sections[section];
    let auto_minus = negative && section == 0;
    let mut output = Output {
        fragments: vec![],
        work,
    };
    let rounded = match &selected.body {
        Body::Literal(tokens) => {
            output.tokens(tokens)?;
            false
        }
        Body::General => {
            let value = general(&n, &d, selected.offset, output.work)?;
            if auto_minus {
                output.text("-")?;
            }
            output.text(&value)?;
            false
        }
        Body::Number(pattern) => number(pattern, n, d, auto_minus, symbols, &mut output)?,
    };
    output.work.poll()?;
    Ok(NumberDisplay {
        section: section as u8,
        color: selected.color,
        fragments: output.fragments,
        rounded,
    })
}
fn general(
    n: &BigUint,
    d: &BigUint,
    offset: usize,
    work: &mut Work<'_>,
) -> Result<String, FormatError> {
    // General selection depends on the eventual available width. Only an exact
    // finite spelling within the standard's eleven-character fixed window is
    // emitted here; scientific/precision selection belongs to measured layout.
    let unresolved = || {
        FormatError::Unresolved(NumberFormatIssue {
            offset: offset as u32,
            reason: NumberFormatReason::GeneralNeedsLayout,
        })
    };
    let mut out = (n / d).to_str_radix(10);
    let mut r = n % d;
    if out.len() > 11 {
        return Err(unresolved());
    }
    if r != BigUint::from(0u8) {
        out.push('.');
    }
    while r != BigUint::from(0u8) {
        work.poll()?;
        if out.len() >= 11 {
            return Err(unresolved());
        }
        r *= 10u8;
        out.push_str(&(&r / d).to_str_radix(10));
        r %= d;
    }
    Ok(out)
}
fn number(
    pattern: &Pattern,
    n: BigUint,
    d: BigUint,
    negative: bool,
    symbols: &NumberSymbols,
    out: &mut Output<'_, '_>,
) -> Result<bool, FormatError> {
    let (n, d) = out.work.scale(n, d, pattern.scale)?;
    let places = pattern
        .fraction
        .iter()
        .filter(|t| matches!(t, Token::Digit(_)))
        .count();
    let mut exponent = 0;
    let (mut digits, mut rounded) = if pattern.exponent.is_some() && n != BigUint::from(0u8) {
        let slots = pattern
            .integer
            .iter()
            .filter(|t| matches!(t, Token::Digit(_)))
            .count() as i32;
        let mut order = n.to_str_radix(10).len() as i32 - d.to_str_radix(10).len() as i32;
        let (scaled_n, scaled_d) = out.work.scale(n.clone(), d.clone(), -order)?;
        if scaled_n < scaled_d {
            order -= 1;
        }
        exponent = order.div_euclid(slots) * slots;
        let (mn, md) = out.work.scale(n.clone(), d.clone(), -exponent)?;
        let (mut digits, mut rounded) = out.work.round(&mn, &md, places)?;
        if digits.len() > slots as usize + places {
            exponent += slots;
            let (mn, md) = out.work.scale(n, d, -exponent)?;
            (digits, rounded) = out.work.round(&mn, &md, places)?;
        }
        (digits, rounded)
    } else {
        out.work.round(&n, &d, places)?
    };
    // Padding is representation only. Negative zero never acquires a visible '-'.
    let nonzero = digits.bytes().any(|b| b != b'0');
    if digits.len() <= places {
        digits = format!("{}{}", "0".repeat(places + 1 - digits.len()), digits);
    }
    let split = digits.len() - places;
    let integer = &digits[..split];
    let fraction = &digits[split..];
    if negative && nonzero {
        out.text("-")?;
    }
    out.tokens(&pattern.prefix)?;
    integral(
        &pattern.integer,
        integer,
        pattern.grouped,
        &symbols.group_separator,
        out,
    )?;
    if pattern.decimal {
        out.text(&symbols.decimal_separator)?;
    }
    fractional(&pattern.fraction, fraction, out)?;
    out.tokens(&pattern.before_exponent)?;
    if let Some(e) = &pattern.exponent {
        out.character(e.letter)?;
        if exponent < 0 {
            out.text("-")?;
        } else if e.plus {
            out.text("+")?;
        }
        let s = exponent.unsigned_abs().to_string();
        out.text(&"0".repeat(e.digits.saturating_sub(s.len())))?;
        out.text(&s)?;
    }
    out.tokens(&pattern.suffix)?;
    // 'rounded' is about numeric value, including a tiny negative rounded to zero.
    if !nonzero && negative {
        rounded = true;
    }
    Ok(rounded)
}
fn integral(
    tokens: &[Token],
    value: &str,
    grouped: bool,
    separator: &str,
    out: &mut Output<'_, '_>,
) -> Result<(), FormatError> {
    let value = if value == "0" { "" } else { value };
    let mut remaining = value.len();
    let mut selected = vec![None; tokens.len()];
    for (i, t) in tokens.iter().enumerate().rev() {
        if let Token::Digit(kind) = t {
            if remaining > 0 {
                remaining -= 1;
                selected[i] = Some(value.as_bytes()[remaining]);
            } else if *kind == '0' {
                selected[i] = Some(b'0');
            }
        }
    }
    let mut count = remaining + selected.iter().filter(|d| d.is_some()).count();
    let write =
        |byte: u8, count: &mut usize, out: &mut Output<'_, '_>| -> Result<(), FormatError> {
            out.character(byte as char)?;
            *count -= 1;
            if grouped && *count > 0 && count.is_multiple_of(3) {
                out.text(separator)?;
            }
            Ok(())
        };
    for b in value[..remaining].bytes() {
        write(b, &mut count, out)?;
    }
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::Digit('?') if selected[i].is_none() => out.reserve('0')?,
            Token::Digit(_) => {
                if let Some(b) = selected[i] {
                    write(b, &mut count, out)?;
                }
            }
            _ => out.token(t)?,
        }
    }
    Ok(())
}
fn fractional(tokens: &[Token], value: &str, out: &mut Output<'_, '_>) -> Result<(), FormatError> {
    let mut position = 0;
    let mut visible = value.bytes().rposition(|b| b != b'0').map_or(0, |i| i + 1);
    for t in tokens {
        if let Token::Digit(c) = t {
            position += 1;
            if *c == '0' {
                visible = visible.max(position);
            }
        }
    }
    position = 0;
    for t in tokens {
        match t {
            Token::Digit(c) => {
                if position < visible {
                    out.character(value.as_bytes()[position] as char)?;
                } else if *c == '?' {
                    out.reserve('0')?;
                }
                position += 1;
            }
            _ => out.token(t)?,
        }
    }
    Ok(())
}
