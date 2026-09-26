use crate::source::{boolean, drawingml::*, fill::SourceFill, integer, malformed};
use mo_common::Emu;
use mo_xml::{Element, XmlError};
use serde::de::DeserializeOwned;

#[derive(Clone, Copy)]
pub(in crate::source) enum Children {
    None,
    OptionalColor,
    Color,
    TwoColors,
    ColorChange,
    Fill,
    Container,
    Effects,
}
#[derive(Default)]
pub(in crate::source) struct Slots {
    pub colors: Vec<SourceColor>,
    pub fill: Option<Box<SourceFill>>,
    pub nodes: Vec<u32>,
}
impl Slots {
    pub fn take_color(&mut self) -> Result<SourceColor, XmlError> {
        if self.colors.is_empty() {
            return Err(malformed("missing effect color"));
        }
        Ok(self.colors.remove(0))
    }
    pub fn take_optional_color(&mut self) -> Result<Option<SourceColor>, XmlError> {
        Ok(if self.colors.is_empty() {
            None
        } else {
            Some(self.colors.remove(0))
        })
    }
    pub fn take_fill(&mut self) -> Result<Box<SourceFill>, XmlError> {
        self.fill
            .take()
            .ok_or_else(|| malformed("missing effect fill"))
    }
    pub fn take_node(&mut self) -> Result<u32, XmlError> {
        if self.nodes.len() != 1 {
            return Err(malformed("effect requires one container"));
        }
        Ok(self.nodes.remove(0))
    }
    pub fn take_nodes(&mut self) -> Result<Vec<u32>, XmlError> {
        Ok(std::mem::take(&mut self.nodes))
    }
    pub fn require_empty(&self) -> Result<(), XmlError> {
        if self.colors.is_empty() && self.fill.is_none() && self.nodes.is_empty() {
            Ok(())
        } else {
            Err(malformed("unexpected effect content"))
        }
    }
}
fn normalize_token(v: &str) -> String {
    let mut result = String::with_capacity(v.len());
    let mut separator = false;
    for c in v.chars() {
        if matches!(c, ' ' | '\t' | '\r' | '\n') {
            separator = !result.is_empty();
        } else {
            if separator {
                result.push(' ');
                separator = false;
            }
            result.push(c);
        }
    }
    result
}

pub(super) fn token(e: &Element, name: &str) -> Result<String, XmlError> {
    Ok(normalize_token(required(e, name)?))
}
pub(super) fn optional_token(e: &Element, name: &str) -> Result<Option<String>, XmlError> {
    Ok(e.attribute(name).map(normalize_token))
}
pub(super) fn optional_enum<T: DeserializeOwned>(
    e: &Element,
    name: &str,
) -> Result<Option<T>, XmlError> {
    e.attribute(name).map(enumeration).transpose()
}
pub(super) fn required_enum<T: DeserializeOwned>(e: &Element, name: &str) -> Result<T, XmlError> {
    enumeration(required(e, name)?)
}
pub(super) fn optional_boolean(e: &Element, name: &str) -> Result<Option<bool>, XmlError> {
    e.attribute(name).map(boolean).transpose()
}
pub(super) fn positive_coordinate(e: &Element, name: &str) -> Result<Emu, XmlError> {
    let n: i64 = integer(e.attribute(name), name)?;
    if !(0..=27_273_042_316_900).contains(&n) {
        return Err(malformed("effect coordinate outside range"));
    }
    Ok(Emu::new(n))
}
pub(super) fn optional_positive_coordinate(
    e: &Element,
    name: &str,
) -> Result<Option<Emu>, XmlError> {
    e.attribute(name)
        .map(|_| positive_coordinate(e, name))
        .transpose()
}
pub(super) fn optional_coordinate(
    e: &Element,
    name: &str,
) -> Result<Option<NativeCoordinate>, XmlError> {
    e.attribute(name)
        .map(|v| v.trim().to_owned().try_into().map_err(malformed))
        .transpose()
}
pub(super) fn optional_positive_angle(e: &Element, name: &str) -> Result<Option<u32>, XmlError> {
    let n: Option<u32> = e
        .attribute(name)
        .map(|v| integer(Some(v), name))
        .transpose()?;
    if n.is_some_and(|v| v >= 21_600_000) {
        return Err(malformed("effect positive angle outside range"));
    }
    Ok(n)
}
pub(super) fn optional_fixed_angle(e: &Element, name: &str) -> Result<Option<i32>, XmlError> {
    let n: Option<i32> = e
        .attribute(name)
        .map(|v| integer(Some(v), name))
        .transpose()?;
    if n.is_some_and(|v| v <= -5_400_000 || v >= 5_400_000) {
        return Err(malformed("effect fixed angle outside range"));
    }
    Ok(n)
}
pub(super) fn optional_positive(
    e: &Element,
    name: &str,
) -> Result<Option<NativePercentage>, XmlError> {
    e.attribute(name).map(positive_percentage).transpose()
}
pub(super) fn optional_percentage(
    e: &Element,
    name: &str,
) -> Result<Option<NativePercentage>, XmlError> {
    e.attribute(name).map(any_percentage).transpose()
}
pub(super) fn optional_fixed(
    e: &Element,
    name: &str,
) -> Result<Option<NativePercentage>, XmlError> {
    e.attribute(name).map(fixed_percentage).transpose()
}
pub(super) fn optional_positive_fixed(
    e: &Element,
    name: &str,
) -> Result<Option<NativePercentage>, XmlError> {
    e.attribute(name).map(positive_fixed_percentage).transpose()
}
pub(super) fn required_positive_fixed(
    e: &Element,
    name: &str,
) -> Result<NativePercentage, XmlError> {
    positive_fixed_percentage(required(e, name)?)
}
