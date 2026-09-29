//! Native endpoint grammar is distinct from property composition. In particular,
//! by-only contributes an offset to the underlying animation stack; from/to
//! replaces its animation offset but keeps the object's original orientation.
use super::*;
use mo_timeline::RotationComposition;

pub(super) fn read<'a>(tree: &'a [Node], anim: &Node) -> Result<(Payload, &'a Node), PptxError> {
    attrs(anim, &["from", "to", "by"])?;
    let angle = |name| -> Result<Option<i32>, PptxError> {
        anim.element
            .attribute(name)
            .map(|value| {
                value
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| crate::value("timing/rotation", "invalid angle"))
            })
            .transpose()
    };
    // Validate even an ignored by value. Office gives an explicit to precedence.
    let (from, to, by) = (angle("from")?, angle("to")?, angle("by")?);
    let (from, to, composition) = match (from, to, by) {
        (from, Some(to), _) => (from.unwrap_or(0), to, RotationComposition::Layout),
        (Some(from), None, Some(by)) => (
            from,
            from.checked_add(by).ok_or_else(|| {
                unsupported("rotation from/by endpoint exceeds the computation angle range")
            })?,
            RotationComposition::Layout,
        ),
        (None, None, Some(by)) => (0, by, RotationComposition::Add),
        _ => {
            return Err(crate::value(
                "timing/rotation",
                "missing rotation destination",
            ));
        }
    };
    Ok((
        Payload::Rotation {
            from,
            to,
            composition,
        },
        single(tree, anim, "cBhvr")?,
    ))
}
