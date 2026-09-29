//! Present flat transform graphs as separate native effects at presentation
//! depth five. The single-child sequence and unbounded, zero-start parallel
//! containers introduce no new gates, cutoffs, repeats or event references.
use super::*;
use mo_presentation_source::timing::TransformPreset;

pub(super) fn write(
    x: &mut Xml,
    timeline: &Timeline,
    objects: &BTreeMap<ObjectId, u32>,
    document: &mo_presentation_model::Document,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    let count =
        u32::try_from(timeline.nodes.len()).map_err(|_| PptxError::Limit("native timing IDs"))?;
    // Reserve root, behavior, three grouping and one effect ID per behavior.
    count
        .checked_mul(2)
        .and_then(|n| n.checked_add(4))
        .ok_or(PptxError::Limit("native timing IDs"))?;
    let ids: BTreeMap<_, _> = timeline
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (&node.id, i as u32 + 2))
        .collect();
    x.raw("<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst><p:seq concurrent=\"0\" nextAc=\"none\" prevAc=\"none\">")?;
    common(x, count + 2, Some("mainSeq"), None)?;
    x.raw("<p:par>")?;
    common(x, count + 3, None, None)?;
    x.raw("<p:par>")?;
    common(x, count + 4, None, None)?;
    for (i, node) in timeline.nodes.iter().enumerate() {
        crate::cancelled(check)?;
        x.raw("<p:par>")?;
        let preset = TransformPreset::for_effect(&node.effect);
        common(
            x,
            count + 5 + i as u32,
            preset.map(|_| "withEffect"),
            preset,
        )?;
        // Keep the behavior ID and all its start/end references unchanged.
        write_behavior(x, node, &ids, objects, document)?;
        x.raw("</p:childTnLst></p:cTn></p:par>")?;
    }
    x.raw("</p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>")
}

fn common(
    x: &mut Xml,
    id: u32,
    node_type: Option<&str>,
    preset: Option<TransformPreset>,
) -> Result<(), PptxError> {
    x.raw("<p:cTn")?;
    x.attr("id", id)?;
    x.raw(" dur=\"indefinite\" restart=\"never\" fill=\"hold\"")?;
    if let Some(kind) = node_type {
        x.attr("nodeType", kind)?;
    }
    if let Some(preset) = preset {
        x.attr("presetID", preset.id())?;
        x.attr("presetClass", preset.class())?;
        x.raw(" presetSubtype=\"0\"")?;
    }
    x.raw("><p:stCondLst><p:cond delay=\"0\"/></p:stCondLst><p:childTnLst>")
}
