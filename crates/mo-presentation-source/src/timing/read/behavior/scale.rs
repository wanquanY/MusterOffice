//! Native scale input grammar and lossless projection to explicit endpoints.
//! Original declarations remain in the source OPC; canonical endpoints are only
//! the computation view, never a rewrite of the imported animation.
use super::*;
use crate::source::drawingml::NativePercentage;
use mo_timeline::MAX_SCALE_MILLI_PERCENT;

const IDENTITY: ScaleValue = ScaleValue {
    x: 100_000,
    y: 100_000,
};
const ZERO: ScaleValue = ScaleValue { x: 0, y: 0 };

pub(super) fn read<'a>(tree: &'a [Node], anim: &Node) -> Result<(Payload, &'a Node), PptxError> {
    // zoomContents still has no mapped rendering semantics.
    attrs(anim, &[])?;
    if !anim.text.trim().is_empty() || anim.children.is_empty() {
        return Err(unsupported("scale requires a common behavior"));
    }
    let behavior = &tree[anim.children[0]];
    if !behavior.element.name.is(P, "cBhvr") {
        return Err(unsupported("scale requires a leading common behavior"));
    }
    let mut points = [None; 3];
    let mut previous = None;
    for &index in &anim.children[1..] {
        let node = &tree[index];
        let rank = ["by", "from", "to"]
            .iter()
            .position(|name| node.element.name.is(P, name))
            .ok_or_else(|| unsupported("unmapped scale child"))?;
        // CT_TLAnimateScaleBehavior is an ordered sequence, not a map. Reject
        // duplicate/out-of-order children even if their numeric values agree.
        if previous.is_some_and(|old| rank <= old) {
            return Err(crate::value("timing/scale", "duplicate or unordered point"));
        }
        points[rank] = Some(point(tree, node)?);
        previous = Some(rank);
    }
    let [by, from, to] = points;
    let (from, to) = match (from, to, by) {
        // Office ignores by in either form with an explicit to. Its syntax is
        // still validated above; no unknown declaration is silently consumed.
        (Some(from), Some(to), _) => (from, to),
        (None, Some(to), _) => (ZERO, to),
        (None, None, Some(by)) => (IDENTITY, by),
        // Office accepts this form, but its relative endpoint needs an
        // independently calibrated computation rule. The current WPS probe
        // disagrees with the candidate from + by - identity interpretation.
        // Preserve it as unmapped rather than certifying guessed endpoints.
        (Some(_), None, Some(_)) => {
            return Err(unsupported("from/by scale semantics require calibration"));
        }
        _ => return Err(crate::value("timing/scale", "missing scale destination")),
    };
    Ok((Payload::Scale { from, to }, behavior))
}

fn point(tree: &[Node], node: &Node) -> Result<ScaleValue, PptxError> {
    attrs(node, &["x", "y"])?;
    empty(tree, node)?;
    let axis = |name| -> Result<u32, PptxError> {
        let lexical = node
            .element
            .attribute(name)
            .ok_or_else(|| unsupported(format!("scale point missing {name}")))?;
        let percent = NativePercentage::try_from(lexical.trim().to_owned())
            .map_err(|e| crate::value("timing/scale", e))?;
        let exact = percent.thousandths().ok_or_else(|| {
            unsupported("scale percentage is not exactly representable in thousandths")
        })?;
        if exact < 0 || exact as u32 > MAX_SCALE_MILLI_PERCENT {
            return Err(crate::value(
                "timing/scale",
                "size percentage exceeds native range",
            ));
        }
        Ok(exact as u32)
    };
    Ok(ScaleValue {
        x: axis("x")?,
        y: axis("y")?,
    })
}
