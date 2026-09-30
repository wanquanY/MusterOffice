use super::*;

pub(super) struct Format {
    pub sections: Vec<Section>,
}
pub(super) struct Section {
    pub offset: usize,
    pub color: Option<NumberFormatColor>,
    pub body: Body,
}
pub(super) enum Body {
    General,
    Literal(Vec<Token>),
    Number(Pattern),
}
#[derive(Clone)]
pub(super) enum Token {
    Digit(char),
    Decimal,
    Comma,
    Percent,
    Literal(char),
    Reserve(char),
    Fill(char),
    Exponent { letter: char, plus: bool },
}
pub(super) struct Exponent {
    pub letter: char,
    pub plus: bool,
    pub digits: usize,
}
pub(super) struct Pattern {
    pub prefix: Vec<Token>,
    pub integer: Vec<Token>,
    pub fraction: Vec<Token>,
    pub suffix: Vec<Token>,
    pub before_exponent: Vec<Token>,
    pub decimal: bool,
    pub grouped: bool,
    pub scale: i32,
    pub exponent: Option<Exponent>,
}
fn issue(offset: usize, reason: NumberFormatReason) -> NumberFormatIssue {
    NumberFormatIssue {
        offset: offset as u32,
        reason,
    }
}
fn syntax(offset: usize) -> NumberFormatIssue {
    issue(offset, NumberFormatReason::InvalidSyntax)
}

pub(super) fn compile(
    code: &str,
    limits: FormatLimits,
) -> Result<Result<Format, NumberFormatIssue>, FormatError> {
    let mut sections = vec![];
    let mut quote = false;
    let mut escape = false;
    let mut start = 0;
    for (i, c) in code.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' | '_' | '*' if !quote => escape = true,
            '"' => quote = !quote,
            ';' if !quote => {
                sections.push((start, &code[start..i]));
                start = i + 1;
            }
            _ => {}
        }
    }
    if quote || escape {
        return Ok(Err(syntax(code.len())));
    }
    sections.push((start, &code[start..]));
    if sections.len() > 4 {
        return Ok(Err(syntax(start)));
    }
    let mut compiled = vec![];
    // The fourth section belongs to text values, which are a separate domain.
    for (offset, part) in sections.into_iter().take(3) {
        let section = match section(part, offset) {
            Ok(s) => s,
            Err(e) => return Ok(Err(e)),
        };
        if let Body::Number(p) = &section.body {
            let digits = p
                .fraction
                .iter()
                .filter(|t| matches!(t, Token::Digit(_)))
                .count();
            if digits > limits.max_fraction_digits {
                return Err(FormatError::Limit("fraction digits"));
            }
        }
        compiled.push(section);
    }
    Ok(Ok(Format { sections: compiled }))
}
fn section(code: &str, offset: usize) -> Result<Section, NumberFormatIssue> {
    let mut color = None;
    let mut begin = 0;
    if code.starts_with('[') {
        let end = code.find(']').ok_or_else(|| syntax(offset))?;
        color = Some(match code[1..end].to_ascii_lowercase().as_str() {
            "black" => NumberFormatColor::Black,
            "blue" => NumberFormatColor::Blue,
            "cyan" => NumberFormatColor::Cyan,
            "green" => NumberFormatColor::Green,
            "magenta" => NumberFormatColor::Magenta,
            "red" => NumberFormatColor::Red,
            "white" => NumberFormatColor::White,
            "yellow" => NumberFormatColor::Yellow,
            _ => return Err(bracket(&code[1..end], offset)),
        });
        begin = end + 1;
    }
    if code[begin..].eq_ignore_ascii_case("General") {
        return Ok(Section {
            offset,
            color,
            body: Body::General,
        });
    }
    let mut chars = code[begin..].char_indices().peekable();
    let mut tokens = vec![];
    while let Some((pos, c)) = chars.next() {
        let at = offset + begin + pos;
        let token = match c {
            '0' | '#' | '?' => Token::Digit(c),
            '.' => Token::Decimal,
            ',' => Token::Comma,
            '%' => Token::Percent,
            '\\' | '_' | '*' => {
                let (_, next) = chars.next().ok_or_else(|| syntax(at))?;
                if c == '*' {
                    // ECMA-376 18.8.31: only the last repetition in a section
                    // survives; it does not turn earlier stars into literals.
                    tokens.retain(|t| !matches!(t, Token::Fill(_)));
                    Token::Fill(next)
                } else if c == '_' {
                    Token::Reserve(next)
                } else {
                    Token::Literal(next)
                }
            }
            '"' => {
                let mut closed = false;
                for (_, next) in chars.by_ref() {
                    if next == '"' {
                        closed = true;
                        break;
                    }
                    tokens.push(Token::Literal(next));
                }
                if !closed {
                    return Err(syntax(at));
                }
                continue;
            }
            '[' => {
                let rest = &code[begin + pos + 1..];
                let end = rest.find(']').ok_or_else(|| syntax(at))?;
                return Err(bracket(&rest[..end], at));
            }
            'E' | 'e' if chars.peek().is_some_and(|(_, c)| matches!(c, '+' | '-')) => {
                let (_, sign) = chars.next().expect("peeked");
                Token::Exponent {
                    letter: c,
                    plus: sign == '+',
                }
            }
            'd' | 'D' | 'm' | 'M' | 'y' | 'Y' | 'h' | 'H' | 's' | 'S' => {
                return Err(issue(at, NumberFormatReason::DateTime));
            }
            '/' => return Err(issue(at, NumberFormatReason::Fraction)),
            '@' => return Err(issue(at, NumberFormatReason::TextPlaceholder)),
            '$' | '+' | '-' | '(' | ')' | ':' | '!' | '^' | '&' | '\'' | '~' | '{' | '}' | '<'
            | '=' | '>' | ' ' | '£' | '¥' | '€' | '¢' => Token::Literal(c),
            _ => return Err(issue(at, NumberFormatReason::UnsupportedToken)),
        };
        tokens.push(token);
    }
    let has_digits = tokens.iter().any(|t| matches!(t, Token::Digit(_)));
    let body = if has_digits {
        Body::Number(pattern(tokens, offset)?)
    } else if tokens
        .iter()
        .any(|t| matches!(t, Token::Decimal | Token::Comma | Token::Exponent { .. }))
    {
        return Err(syntax(offset));
    } else {
        Body::Literal(tokens)
    };
    Ok(Section {
        offset,
        color,
        body,
    })
}
fn bracket(s: &str, offset: usize) -> NumberFormatIssue {
    let reason = if s.starts_with(['<', '>', '=']) {
        NumberFormatReason::ConditionalSection
    } else if s.starts_with('$') {
        NumberFormatReason::LocaleDirective
    } else if s.to_ascii_lowercase().starts_with("color") {
        NumberFormatReason::IndexedColor
    } else if s
        .chars()
        .all(|c| matches!(c, 'h' | 'H' | 'm' | 'M' | 's' | 'S'))
    {
        NumberFormatReason::DateTime
    } else {
        NumberFormatReason::UnsupportedToken
    };
    issue(offset, reason)
}
fn pattern(mut tokens: Vec<Token>, offset: usize) -> Result<Pattern, NumberFormatIssue> {
    let scale_percent = 2 * tokens
        .iter()
        .filter(|t| matches!(t, Token::Percent))
        .count() as i32;
    let mut exponent = None;
    let mut exponent_suffix = vec![];
    if let Some(i) = tokens
        .iter()
        .position(|t| matches!(t, Token::Exponent { .. }))
    {
        let Token::Exponent { letter, plus } = tokens[i] else {
            unreachable!()
        };
        let end = i
            + 1
            + tokens[i + 1..]
                .iter()
                .take_while(|t| matches!(t, Token::Digit('0' | '#')))
                .count();
        if end == i + 1
            || tokens[end..].iter().any(|t| {
                matches!(
                    t,
                    Token::Digit(_) | Token::Decimal | Token::Comma | Token::Exponent { .. }
                )
            })
        {
            return Err(syntax(offset));
        }
        exponent = Some(Exponent {
            letter,
            plus,
            digits: end - i - 1,
        });
        exponent_suffix = tokens[end..].to_vec();
        tokens.truncate(i);
    }
    let first = tokens
        .iter()
        .position(|t| matches!(t, Token::Digit(_) | Token::Decimal))
        .ok_or_else(|| syntax(offset))?;
    let last = tokens
        .iter()
        .rposition(|t| matches!(t, Token::Digit(_) | Token::Decimal))
        .ok_or_else(|| syntax(offset))?;
    let mut end = last + 1;
    while tokens.get(end).is_some_and(|t| matches!(t, Token::Comma)) {
        end += 1;
    }
    let mut scale_commas = end - last - 1;
    let prefix = tokens[..first].to_vec();
    let (before_exponent, suffix) = if exponent.is_some() {
        (tokens[end..].to_vec(), exponent_suffix)
    } else {
        (vec![], tokens[end..].to_vec())
    };
    if prefix
        .iter()
        .chain(&suffix)
        .chain(&before_exponent)
        .any(|t| matches!(t, Token::Decimal | Token::Comma | Token::Exponent { .. }))
    {
        return Err(syntax(offset));
    }
    let core = &tokens[first..last + 1];
    let decimal = core.iter().position(|t| matches!(t, Token::Decimal));
    let split = decimal.unwrap_or(core.len());
    let mut integer = core[..split].to_vec();
    while integer.last().is_some_and(|t| matches!(t, Token::Comma)) {
        integer.pop();
        scale_commas += 1;
    }
    let fraction = decimal.map_or_else(Vec::new, |i| core[i + 1..].to_vec());
    if fraction
        .iter()
        .any(|t| matches!(t, Token::Decimal | Token::Comma | Token::Exponent { .. }))
    {
        return Err(syntax(offset));
    }
    let grouped = integer.iter().any(|t| matches!(t, Token::Comma));
    if grouped
        && integer
            .iter()
            .any(|t| !matches!(t, Token::Digit(_) | Token::Comma))
    {
        return Err(issue(offset, NumberFormatReason::GroupedEmbeddedLiteral));
    }
    if grouped
        && (integer.first().is_some_and(|t| matches!(t, Token::Comma))
            || integer
                .windows(2)
                .any(|t| matches!(t, [Token::Comma, Token::Comma])))
    {
        return Err(syntax(offset));
    }
    integer.retain(|t| !matches!(t, Token::Comma));
    if exponent.is_some() && !integer.iter().any(|t| matches!(t, Token::Digit(_))) {
        return Err(syntax(offset));
    }
    Ok(Pattern {
        prefix,
        integer,
        fraction,
        suffix,
        before_exponent,
        decimal: decimal.is_some(),
        grouped,
        scale: scale_percent - 3 * scale_commas as i32,
        exponent,
    })
}
