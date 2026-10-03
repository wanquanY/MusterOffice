//! Exact nonvisual identity extensions. Source bytes are retained unchanged;
//! recognition never suppresses unknown sibling/child elements or attributes.
use crate::{A, P};
use mo_xml::{Element, ExpandedName, XmlError};

const SLIDE_URI: &str = "{BB962C8B-B14F-4D97-AF65-F5344CB8AC3E}";
const OBJECT_URI: &str = "{FF2B5EF4-FFF2-40B4-BE49-F238E27FC236}";

#[derive(Default)]
pub(super) struct Reader {
    slide: Option<usize>,
    object: Option<usize>,
}

impl Reader {
    pub fn end(&mut self, depth: usize) {
        if self.slide == Some(depth) {
            self.slide = None;
        }
        if self.object == Some(depth) {
            self.object = None;
        }
    }

    pub fn allowed(
        &mut self,
        e: &Element,
        stack: &[ExpandedName],
        owner: Option<(usize, usize)>,
    ) -> Result<Option<&'static [&'static str]>, XmlError> {
        let depth = stack.len();
        let parent_is = |ns, local| stack.last().is_some_and(|n| n.is(ns, local));
        if depth == 2 && parent_is(P, "cSld") && e.name.is(P, "extLst") {
            return Ok(Some(&[]));
        }
        if depth == 3
            && stack[1].is(P, "cSld")
            && parent_is(P, "extLst")
            && e.name.is(P, "ext")
            && e.attribute("uri") == Some(SLIDE_URI)
        {
            self.slide = Some(depth);
            return Ok(Some(&["uri"]));
        }
        if self.slide.is_some_and(|d| depth == d + 1)
            && e.name.is(
                "http://schemas.microsoft.com/office/powerpoint/2010/main",
                "creationId",
            )
        {
            if e.attribute("val")
                .and_then(|v| v.parse::<u32>().ok())
                .is_none()
            {
                return Err(super::super::malformed("invalid slide creation identifier"));
            }
            return Ok(Some(&["val"]));
        }
        // The accessibility reader owns the cNvPr/extLst container.
        if owner.is_some_and(|(_, d)| depth == d + 4)
            && stack[depth - 2].is(P, "cNvPr")
            && parent_is(A, "extLst")
            && e.name.is(A, "ext")
            && e.attribute("uri") == Some(OBJECT_URI)
        {
            self.object = Some(depth);
            return Ok(Some(&["uri"]));
        }
        if self.object.is_some_and(|d| depth == d + 1)
            && e.name.is(
                "http://schemas.microsoft.com/office/drawing/2014/main",
                "creationId",
            )
        {
            if !guid(e.attribute("id").unwrap_or("")) {
                return Err(super::super::malformed(
                    "invalid object creation identifier",
                ));
            }
            return Ok(Some(&["id"]));
        }
        Ok(None)
    }
}

fn guid(value: &str) -> bool {
    value.len() == 38
        && value.starts_with('{')
        && value.ends_with('}')
        && value.as_bytes()[1..37].iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
