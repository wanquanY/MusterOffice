//! Deliberately versioned computation policy, not an Office/WPS conformance
//! claim. Unicode script itemization is separate from DrawingML font slots.
use mo_pptx::source::text::fonts::NativeFontSlot;

pub(super) fn select(
    script: &str,
    language: Option<&str>,
    alternative: Option<&str>,
) -> Option<(NativeFontSlot, Option<String>)> {
    use NativeFontSlot::*;
    let (slot, key) = match script {
        "Latn" | "Grek" | "Cyrl" | "Zyyy" | "Zinh" => (Latin, Some(script.into())),
        "Hani" => (EastAsian, han(language).or_else(|| han(alternative))),
        "Hira" | "Kana" => (EastAsian, Some("Jpan".into())),
        "Hang" => (EastAsian, Some("Hang".into())),
        "Bopo" => (EastAsian, Some("Hant".into())),
        "Arab" | "Hebr" | "Syrc" | "Thaa" | "Nkoo" | "Deva" | "Beng" | "Guru" | "Gujr" | "Orya"
        | "Taml" | "Telu" | "Knda" | "Mlym" | "Sinh" | "Thai" | "Laoo" | "Tibt" | "Mymr"
        | "Khmr" => (ComplexScript, Some(script.into())),
        _ => return None,
    };
    Some((slot, key))
}
fn han(language: Option<&str>) -> Option<String> {
    let language = language?;
    if language.is_empty() || language.len() > 255 {
        return None;
    }
    let language = language.to_ascii_lowercase();
    let parts: Vec<_> = language.split('-').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || p.len() > 8 || !p.bytes().all(|b| b.is_ascii_alphanumeric()))
    {
        return None;
    }
    let primary = parts[0];
    if !matches!(primary, "ja" | "ko" | "zh") {
        return None;
    }
    let mut at = 1;
    // Read only the language's leading extlang/script/region positions. Never
    // scan variant, extension or private-use payload for script-like strings.
    for _ in 0..3 {
        if parts
            .get(at)
            .is_some_and(|p| p.len() == 3 && p.bytes().all(|b| b.is_ascii_alphabetic()))
        {
            at += 1;
        } else {
            break;
        }
    }
    let script = parts
        .get(at)
        .copied()
        .filter(|p| p.len() == 4 && p.bytes().all(|b| b.is_ascii_alphabetic()));
    if script.is_some() {
        at += 1;
    }
    let region = parts.get(at).copied().filter(|p| {
        p.len() == 2 && p.bytes().all(|b| b.is_ascii_alphabetic())
            || p.len() == 3 && p.bytes().all(|b| b.is_ascii_digit())
    });
    match primary {
        "ja" if script.is_none_or(|s| matches!(s, "jpan" | "hani" | "hira" | "kana")) => {
            Some("Jpan".into())
        }
        "ko" if script.is_none_or(|s| matches!(s, "kore" | "hang" | "hani")) => Some("Hang".into()),
        "zh" => match script {
            Some("hans") => Some("Hans".into()),
            Some("hant") => Some("Hant".into()),
            None | Some("hani") => match region {
                Some("cn" | "sg") => Some("Hans".into()),
                Some("tw" | "hk" | "mo") => Some("Hant".into()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_han_language_and_extension_boundaries() {
        for (lang, expected) in [
            ("zh-CN", Some("Hans")),
            ("zh-Hant-CN", Some("Hant")),
            ("zh-TW", Some("Hant")),
            ("ZH-sg", Some("Hans")),
            ("ja", Some("Jpan")),
            ("ko-KR", Some("Hang")),
            ("zh", None),
            ("en", None),
            ("zh-x-Hant", None),
            ("zh-u-rg-twzzzz", None),
            ("zh-Latn-TW", None),
            ("zh-pinyin-TW", None),
            ("zh--Hant", None),
            ("ja-Latn", None),
        ] {
            assert_eq!(han(Some(lang)).as_deref(), expected);
        }
        assert_eq!(
            select("Hani", Some("en"), Some("zh-TW"))
                .unwrap()
                .1
                .as_deref(),
            Some("Hant")
        );
        assert_eq!(select("Hani", None, None).unwrap().1, None);
        assert_eq!(
            select("Arab", Some("en"), None).unwrap().0,
            NativeFontSlot::ComplexScript
        );
        assert!(select("Zzzz", None, None).is_none());
        assert!(select("Dsrt", None, None).is_none());
    }
}
