//! Nonvisual accessibility semantics shared by export inspection and import.
use super::{SourceSurface, malformed};
use crate::{A, P};
use mo_xml::{Element, ExpandedName, XmlError};
use std::collections::BTreeSet;

pub(super) const DECORATIVE_NS: &str =
    "http://schemas.microsoft.com/office/drawing/2017/decorative";
const DECORATIVE_URI: &str = "{C183D7F6-B498-43B3-948B-1728B52AA6E4}";
#[derive(Default)]
pub(super) struct Reader {
    active: Option<(usize, usize, bool)>,
    seen: BTreeSet<usize>,
}
impl Reader {
    pub fn start(
        &mut self,
        e: &Element,
        stack: &[ExpandedName],
        owner: Option<(usize, usize)>,
        surface: &mut SourceSurface,
    ) -> Result<bool, XmlError> {
        let depth = stack.len();
        let Some((index, object_depth)) = owner else {
            return Ok(false);
        };
        if depth == object_depth + 3
            && stack.last().is_some_and(|n| n.is(P, "cNvPr"))
            && e.name.is(A, "extLst")
        {
            attributes(e, &[])?;
            return Ok(true);
        }
        if depth == object_depth + 4
            && stack[depth - 2].is(P, "cNvPr")
            && stack[depth - 1].is(A, "extLst")
            && e.name.is(A, "ext")
            && e.attribute("uri") == Some(DECORATIVE_URI)
        {
            attributes(e, &["uri"])?;
            if !self.seen.insert(index) {
                return Err(malformed("duplicate decorative extension"));
            }
            self.active = Some((depth, index, false));
            return Ok(true);
        }
        if let Some((extension_depth, object, seen)) = &mut self.active
            && depth == *extension_depth + 1
            && e.name.is(DECORATIVE_NS, "decorative")
        {
            attributes(e, &["val"])?;
            if *seen {
                return Err(malformed("duplicate decorative value"));
            }
            surface.objects[*object].accessibility.decorative = match e.attribute("val") {
                Some("1" | "true") => true,
                Some("0" | "false") => false,
                _ => return Err(malformed("invalid decorative value")),
            };
            *seen = true;
            return Ok(true);
        }
        Ok(false)
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if let Some((at, _, seen)) = self.active
            && at == depth
        {
            if !seen {
                return Err(malformed("missing decorative value"));
            }
            self.active = None;
        }
        Ok(())
    }
}
fn attributes(e: &Element, allowed: &[&str]) -> Result<(), XmlError> {
    if e.attributes
        .iter()
        .any(|a| !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()))
    {
        return Err(malformed("unsupported accessibility attribute"));
    }
    Ok(())
}
