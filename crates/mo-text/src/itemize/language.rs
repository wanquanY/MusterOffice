//! Bounded authored-language hints for the draft itemization policy. BCP 47
//! script subtags are explicit evidence; a small documented CJK policy handles
//! their multi-script language codes. No likely-subtag database or host locale.
use super::{LanguageSpan, Script, TextError, cancelled};

pub(super) struct Run {
    end: u32,
    script: Option<Script>,
}

pub(super) fn prepare(
    spans: &[LanguageSpan<'_>],
    count: u32,
    maximum: usize,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Run>, TextError> {
    if spans.len() > maximum {
        return Err(TextError::Limit("itemization language runs"));
    }
    let mut end = 0;
    let mut runs = Vec::with_capacity(spans.len());
    for span in spans {
        cancelled(check)?;
        if span.end <= end || span.end > count {
            return Err(TextError::Invalid("exhaustive itemization language runs"));
        }
        if span.language.len() > 255 {
            return Err(TextError::Limit("itemization language bytes"));
        }
        runs.push(Run {
            end: span.end,
            script: hint(span.language),
        });
        end = span.end;
    }
    if !spans.is_empty() && end != count {
        return Err(TextError::Invalid("exhaustive itemization language runs"));
    }
    Ok(runs)
}

pub(super) fn for_cluster(
    runs: &[Run],
    at: &mut usize,
    start: u32,
    end: u32,
    check: &dyn Fn() -> bool,
) -> Result<Option<Script>, TextError> {
    if runs.is_empty() {
        return Ok(None);
    }
    while runs[*at].end <= start {
        cancelled(check)?;
        *at += 1;
    }
    let mut last = *at;
    let hint = runs[last].script;
    while runs[last].end < end {
        cancelled(check)?;
        last += 1;
        if runs[last].script != hint {
            return Ok(None);
        }
    }
    Ok(hint)
}

fn letters(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Malformed/unsupported language metadata supplies no evidence. It is not an
/// excuse to override Script_Extensions or to suppress AmbiguousScript notices.
fn hint(language: &str) -> Option<Script> {
    if language.is_empty() || language.len() > 255 {
        return None;
    }
    let lower = language.to_ascii_lowercase();
    let parts: Vec<_> = lower.split('-').collect();
    if parts
        .iter()
        .any(|s| s.is_empty() || s.len() > 8 || !s.bytes().all(|b| b.is_ascii_alphanumeric()))
    {
        return None;
    }
    let primary = parts[0];
    if !(2..=8).contains(&primary.len()) || !letters(primary) {
        return None;
    }
    let mut at = 1;
    if primary.len() <= 3 {
        for _ in 0..3 {
            if parts.get(at).is_some_and(|s| s.len() == 3 && letters(s)) {
                at += 1;
            } else {
                break;
            }
        }
    }
    let explicit = parts
        .get(at)
        .copied()
        .filter(|s| s.len() == 4 && letters(s));
    if explicit.is_some() {
        at += 1;
    }
    if parts.get(at).is_some_and(|s| {
        s.len() == 2 && letters(s) || s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit())
    }) {
        at += 1;
    }
    let variant_start = at;
    while parts.get(at).is_some_and(|s| {
        (5..=8).contains(&s.len()) || s.len() == 4 && s.as_bytes()[0].is_ascii_digit()
    }) {
        if parts[variant_start..at].contains(&parts[at]) {
            return None;
        }
        at += 1;
    }
    let mut singleton_mask = 0_u64;
    while at < parts.len() {
        let singleton = parts[at];
        if singleton.len() != 1 {
            return None;
        }
        at += 1;
        if singleton == "x" {
            return (at < parts.len())
                .then(|| selected(primary, explicit))
                .flatten();
        }
        let key = u32::from(singleton.as_bytes()[0]);
        let bit = if key <= u32::from(b'9') {
            key - u32::from(b'0')
        } else {
            key - u32::from(b'a') + 10
        };
        if singleton_mask & (1_u64 << bit) != 0 {
            return None;
        }
        singleton_mask |= 1_u64 << bit;
        let start = at;
        while parts.get(at).is_some_and(|s| s.len() >= 2) {
            at += 1;
        }
        if at == start {
            return None;
        }
    }
    selected(primary, explicit)
}

fn selected(primary: &str, explicit: Option<&str>) -> Option<Script> {
    let script = match explicit {
        Some("hans" | "hant") => "Hani".into(),
        Some("jpan") => "Kana".into(),
        Some("kore") => "Hang".into(),
        Some(s) => {
            let mut canonical = s.to_owned();
            canonical[..1].make_ascii_uppercase();
            canonical
        }
        None => match primary {
            "zh" => "Hani".into(),
            "ja" => "Kana".into(),
            "ko" => "Hang".into(),
            _ => return None,
        },
    };
    Script::from_tag(&script).filter(|s| !s.is_contextual() && *s != Script::unknown())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_valid_leading_script_metadata_and_documented_cjk_codes_supply_hints() {
        for (language, script) in [
            ("zh", Some("Hani")),
            ("ZH-Hant-TW", Some("Hani")),
            ("zh-Latn-CN", Some("Latn")),
            ("ja", Some("Kana")),
            ("ko-KR", Some("Hang")),
            ("und-Hira", Some("Hira")),
            ("en-Hani-US-u-ca-chinese-x-private", Some("Hani")),
            ("en-x-Hani", None),
            ("en-u-sc-hani", None),
            ("und", None),
            ("zh--Hant", None),
            ("zh-Hant-u", None),
            ("zh-Hant-x", None),
            ("zh-Hant-u-ca-chinese-u-nu-hanidec", None),
            ("zh-Hant-fonipa-fonipa", None),
            ("en-Zzzz", None),
            ("en-Zyyy", None),
            ("en-Abcd", None),
        ] {
            assert_eq!(hint(language).map(Script::tag), script, "{language}");
        }
    }
}
