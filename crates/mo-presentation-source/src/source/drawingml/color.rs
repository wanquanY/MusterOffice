use super::names::*;
use crate::source::{integer, malformed};
use mo_xml::{Element, XmlError};
use schemars::JsonSchema;
use schemars::{Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Exact native percentage spelling. Integer form is in 1/1000 percent;
/// decimal percent form can exceed the domain model's eventual fixed precision.
/// No floating-point parsing or rounding occurs in the source representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NativePercentage(String);
impl NativePercentage {
    pub fn lexical(&self) -> &str {
        &self.0
    }
    pub(crate) fn positive_fixed(value: &str) -> Result<Self, XmlError> {
        percentage(value.trim(), Range::PositiveFixed)
    }
    /// Exact bounded projection for domains using native thousandths of a
    /// percent. Preserve the source spelling even when projection is impossible.
    pub(crate) fn thousandths(&self) -> Option<i32> {
        let Some(decimal) = self.0.strip_suffix('%') else {
            return self.0.parse().ok();
        };
        let negative = decimal.starts_with('-');
        let unsigned = decimal.strip_prefix('-').unwrap_or(decimal);
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        let fraction = fraction.trim_end_matches('0');
        if fraction.len() > 3 {
            return None;
        }
        let mut value = whole.parse::<i64>().ok()?.checked_mul(1000)?;
        if !fraction.is_empty() {
            value = value.checked_add(
                fraction
                    .parse::<i64>()
                    .ok()?
                    .checked_mul(10i64.pow(3 - fraction.len() as u32))?,
            )?;
        }
        i32::try_from(if negative { -value } else { value }).ok()
    }
}
impl TryFrom<String> for NativePercentage {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        percentage(&value, Range::Any).map_err(|e| e.to_string())?;
        Ok(Self(value))
    }
}
impl From<NativePercentage> for String {
    fn from(value: NativePercentage) -> Self {
        value.0
    }
}
impl JsonSchema for NativePercentage {
    fn schema_name() -> Cow<'static, str> {
        "NativePercentage".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string", "pattern":"^([+-]?[0-9]+|-?[0-9]+(\\.[0-9]+)?%)$", "not":{"pattern":"[^0-9+.%\\-]"}, "description":"Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use."})
    }
}
#[derive(Clone, Copy)]
enum Range {
    Any,
    Positive,
    Fixed,
    PositiveFixed,
}
fn percentage(value: &str, range: Range) -> Result<NativePercentage, XmlError> {
    let invalid = || malformed("invalid native percentage or range");
    if let Some(decimal) = value.strip_suffix('%') {
        let negative = decimal.starts_with('-');
        let unsigned = decimal.strip_prefix('-').unwrap_or(decimal);
        let (whole, fraction) = unsigned
            .split_once('.')
            .map_or((unsigned, None), |(a, b)| (a, Some(b)));
        if whole.is_empty()
            || !whole.bytes().all(|c| c.is_ascii_digit())
            || fraction.is_some_and(|f| f.is_empty() || !f.bytes().all(|c| c.is_ascii_digit()))
        {
            return Err(invalid());
        }
        if negative && matches!(range, Range::Positive | Range::PositiveFixed) {
            return Err(invalid());
        }
        if matches!(range, Range::Fixed | Range::PositiveFixed) {
            // Match fixed-percent lexical restrictions and semantic bounds.
            if (whole.len() > 2 && whole != "100") || fraction.is_some_and(|f| f.len() > 2) {
                return Err(invalid());
            }
            let n: u16 = whole.parse().map_err(|_| invalid())?;
            if n > 100 || (n == 100 && fraction.is_some_and(|f| f.bytes().any(|c| c != b'0'))) {
                return Err(invalid());
            }
        }
    } else {
        let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
        if unsigned.is_empty() || !unsigned.bytes().all(|c| c.is_ascii_digit()) {
            return Err(invalid());
        }
        let number = value.parse::<i32>().map_err(|_| invalid())?;
        if matches!(range, Range::Positive | Range::PositiveFixed) && number < 0
            || matches!(range, Range::Fixed | Range::PositiveFixed)
                && !(-100_000..=100_000).contains(&number)
        {
            return Err(invalid());
        }
    }
    Ok(NativePercentage(value.into()))
}

pub(in crate::source) fn positive_percentage(value: &str) -> Result<NativePercentage, XmlError> {
    percentage(value.trim(), Range::Positive)
}
pub(in crate::source) fn positive_fixed_percentage(
    value: &str,
) -> Result<NativePercentage, XmlError> {
    NativePercentage::positive_fixed(value)
}
pub(in crate::source) fn fixed_percentage(value: &str) -> Result<NativePercentage, XmlError> {
    percentage(value.trim(), Range::Fixed)
}
pub(in crate::source) fn any_percentage(value: &str) -> Result<NativePercentage, XmlError> {
    percentage(value.trim(), Range::Any)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceColorValue {
    Srgb {
        rgb: [u8; 3],
    },
    ScRgb {
        red: NativePercentage,
        green: NativePercentage,
        blue: NativePercentage,
    },
    Hsl {
        hue: i32,
        saturation: NativePercentage,
        luminance: NativePercentage,
    },
    System {
        color: SystemColor,
        last_color: Option<[u8; 3]>,
    },
    Scheme {
        slot: SchemeColor,
    },
    Preset {
        color: PresetColor,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum SourceColorTransform {
    Tint(NativePercentage),
    Shade(NativePercentage),
    #[serde(rename = "comp")]
    Complement,
    #[serde(rename = "inv")]
    Inverse,
    Gray,
    Alpha(NativePercentage),
    AlphaOff(NativePercentage),
    AlphaMod(NativePercentage),
    Hue(i32),
    HueOff(i32),
    HueMod(NativePercentage),
    Sat(NativePercentage),
    SatOff(NativePercentage),
    SatMod(NativePercentage),
    Lum(NativePercentage),
    LumOff(NativePercentage),
    LumMod(NativePercentage),
    Red(NativePercentage),
    RedOff(NativePercentage),
    RedMod(NativePercentage),
    Green(NativePercentage),
    GreenOff(NativePercentage),
    GreenMod(NativePercentage),
    Blue(NativePercentage),
    BlueOff(NativePercentage),
    BlueMod(NativePercentage),
    Gamma,
    InvGamma,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceColor {
    pub source_ordinal: u32,
    pub value: SourceColorValue,
    /// XML application order, including repeated transforms.
    pub transforms: Vec<SourceColorTransform>,
}

impl SourceColorValue {
    pub(in crate::source) fn percentages(&self) -> impl Iterator<Item = &NativePercentage> {
        let values = match self {
            Self::ScRgb { red, green, blue } => [Some(red), Some(green), Some(blue)],
            Self::Hsl {
                saturation,
                luminance,
                ..
            } => [Some(saturation), Some(luminance), None],
            _ => [None, None, None],
        };
        values.into_iter().flatten()
    }
}
impl SourceColorTransform {
    pub(in crate::source) fn percentage(&self) -> Option<&NativePercentage> {
        use SourceColorTransform::*;
        match self {
            Tint(p) | Shade(p) | Alpha(p) | AlphaOff(p) | AlphaMod(p) | HueMod(p) | Sat(p)
            | SatOff(p) | SatMod(p) | Lum(p) | LumOff(p) | LumMod(p) | Red(p) | RedOff(p)
            | RedMod(p) | Green(p) | GreenOff(p) | GreenMod(p) | Blue(p) | BlueOff(p)
            | BlueMod(p) => Some(p),
            Complement | Inverse | Gray | Hue(_) | HueOff(_) | Gamma | InvGamma => None,
        }
    }
}

pub(in crate::source) fn required<'a>(e: &'a Element, name: &str) -> Result<&'a str, XmlError> {
    e.attribute(name)
        .ok_or_else(|| malformed(format!("missing theme attribute {name}")))
}
pub(in crate::source) fn enumeration<T: serde::de::DeserializeOwned>(
    value: &str,
) -> Result<T, XmlError> {
    T::deserialize(serde::de::value::StrDeserializer::<serde::de::value::Error>::new(value.trim()))
        .map_err(|_| malformed("invalid native theme enumeration"))
}
fn rgb(value: &str) -> Result<[u8; 3], XmlError> {
    let value = value.trim();
    if value.len() != 6 || !value.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(malformed("invalid RGB hex color"));
    }
    Ok([
        u8::from_str_radix(&value[0..2], 16).unwrap(),
        u8::from_str_radix(&value[2..4], 16).unwrap(),
        u8::from_str_radix(&value[4..6], 16).unwrap(),
    ])
}
fn angle(e: &Element, name: &str, positive_fixed: bool) -> Result<i32, XmlError> {
    let n = integer(e.attribute(name), "color angle")?;
    if positive_fixed && !(0..21_600_000).contains(&n) {
        return Err(malformed("color angle outside range"));
    }
    Ok(n)
}
pub(in crate::source) fn color(e: &Element, ordinal: u32) -> Result<SourceColor, XmlError> {
    let pct = |name| percentage(required(e, name)?.trim(), Range::Any);
    let value = match e.name.local.as_str() {
        "srgbClr" => SourceColorValue::Srgb {
            rgb: rgb(required(e, "val")?)?,
        },
        "scrgbClr" => SourceColorValue::ScRgb {
            red: pct("r")?,
            green: pct("g")?,
            blue: pct("b")?,
        },
        "hslClr" => SourceColorValue::Hsl {
            hue: angle(e, "hue", true)?,
            saturation: pct("sat")?,
            luminance: pct("lum")?,
        },
        "sysClr" => SourceColorValue::System {
            color: enumeration(required(e, "val")?)?,
            last_color: e.attribute("lastClr").map(rgb).transpose()?,
        },
        "schemeClr" => SourceColorValue::Scheme {
            slot: enumeration(required(e, "val")?)?,
        },
        "prstClr" => SourceColorValue::Preset {
            color: enumeration(required(e, "val")?)?,
        },
        _ => {
            return Err(XmlError::Compatibility(
                "unknown theme color representation".into(),
            ));
        }
    };
    Ok(SourceColor {
        source_ordinal: ordinal,
        value,
        transforms: Vec::new(),
    })
}
pub(in crate::source) fn transform(e: &Element) -> Result<SourceColorTransform, XmlError> {
    use SourceColorTransform::*;
    let pct = |range| percentage(required(e, "val")?.trim(), range);
    let empty = |v| {
        if e.attribute("val").is_none() {
            Ok(v)
        } else {
            Err(malformed("unexpected color transform value"))
        }
    };
    match e.name.local.as_str() {
        "tint" => Ok(Tint(pct(Range::PositiveFixed)?)),
        "shade" => Ok(Shade(pct(Range::PositiveFixed)?)),
        "comp" => empty(Complement),
        "inv" => empty(Inverse),
        "gray" => empty(Gray),
        "gamma" => empty(Gamma),
        "invGamma" => empty(InvGamma),
        "alpha" => Ok(Alpha(pct(Range::PositiveFixed)?)),
        "alphaOff" => Ok(AlphaOff(pct(Range::Fixed)?)),
        "alphaMod" => Ok(AlphaMod(pct(Range::Positive)?)),
        "hue" => Ok(Hue(angle(e, "val", true)?)),
        "hueOff" => Ok(HueOff(angle(e, "val", false)?)),
        "hueMod" => Ok(HueMod(pct(Range::Positive)?)),
        "sat" => Ok(Sat(pct(Range::Any)?)),
        "satOff" => Ok(SatOff(pct(Range::Any)?)),
        "satMod" => Ok(SatMod(pct(Range::Any)?)),
        "lum" => Ok(Lum(pct(Range::Any)?)),
        "lumOff" => Ok(LumOff(pct(Range::Any)?)),
        "lumMod" => Ok(LumMod(pct(Range::Any)?)),
        "red" => Ok(Red(pct(Range::Any)?)),
        "redOff" => Ok(RedOff(pct(Range::Any)?)),
        "redMod" => Ok(RedMod(pct(Range::Any)?)),
        "green" => Ok(Green(pct(Range::Any)?)),
        "greenOff" => Ok(GreenOff(pct(Range::Any)?)),
        "greenMod" => Ok(GreenMod(pct(Range::Any)?)),
        "blue" => Ok(Blue(pct(Range::Any)?)),
        "blueOff" => Ok(BlueOff(pct(Range::Any)?)),
        "blueMod" => Ok(BlueMod(pct(Range::Any)?)),
        _ => Err(XmlError::Compatibility(
            "unknown theme color transform".into(),
        )),
    }
}
