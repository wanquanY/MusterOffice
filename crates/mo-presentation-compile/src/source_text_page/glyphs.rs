use super::*;
use mo_presentation_source::source::{color::ColorSample, text::paint::TextPaintError};
use mo_text::fallback::FontFragment;
#[derive(PartialEq, Eq, Clone, Copy)]
pub(super) struct Color {
    key: Option<[u64; 4]>,
    rgba: Option<[u8; 4]>,
}
#[derive(PartialEq, Eq, Clone, Copy)]
pub(super) struct RunColors {
    fill: Color,
    underline: Option<Color>,
    strike: bool,
}
pub(super) fn colors(paint: &TextRunPaint) -> Result<RunColors, TextPaintError> {
    let fill = color(&paint.fill)?;
    let underline = match &paint.underline {
        None => None,
        Some(paint::UnderlinePaint::FollowText { .. }) => Some(fill),
        Some(paint::UnderlinePaint::Independent { fill, .. }) => Some(color(fill)?),
    };
    Ok(RunColors {
        fill,
        underline,
        strike: paint.strike,
    })
}
pub(super) fn color(paint: &TextPaint) -> Result<Color, TextPaintError> {
    match paint {
        TextPaint::None { .. } => Ok(Color {
            key: None,
            rgba: None,
        }),
        TextPaint::Solid {
            color: ColorSample::Resolved { srgb, rgba8, .. },
            ..
        } => Ok(Color {
            key: Some(srgb.map(f64::to_bits)),
            rgba: Some(*rgba8),
        }),
        TextPaint::Solid {
            declaration,
            color: ColorSample::Unresolved { reason },
            ..
        } => Err(TextPaintError::Color {
            declaration: Box::new(declaration.clone()),
            reason: reason.clone(),
        }),
    }
}
pub(super) fn bind(
    frame: &SourceFramePlan,
    paints: &[Vec<TextRunPaint>],
    check: &dyn Fn() -> bool,
) -> Result<(Vec<GlyphClusterPaint>, Vec<u32>), SourcePageError> {
    // One sorted cluster map per fragment; no quadratic next-cluster scans.
    let mut ends = BTreeMap::new();
    for (p, paragraph) in frame.paragraphs.iter().enumerate() {
        let shaped = &paragraph
            .computed
            .geometry
            .paths
            .layout
            .geometry
            .as_ref()
            .expect("frame geometry")
            .shaping;
        for (i, item) in shaped.fallback.items.iter().enumerate() {
            for (j, fragment) in item.fragments.iter().enumerate() {
                cancel(check)?;
                let FontFragment::Selected {
                    start, end, shaped, ..
                } = fragment
                else {
                    unreachable!("complete frame")
                };
                let mut clusters: Vec<_> =
                    shaped.runs[0].glyphs.iter().map(|g| g.cluster).collect();
                clusters.push(*end);
                clusters.sort_unstable();
                clusters.dedup();
                if clusters.first().is_some_and(|n| n < start) {
                    return Err(SourcePageError::Invalid("glyph cluster before fragment"));
                }
                for pair in clusters.windows(2) {
                    if pair[0] >= pair[1] || pair[1] > *end {
                        return Err(SourcePageError::Invalid("glyph cluster range"));
                    }
                    ends.insert((p as u32, i as u32, j as u32, pair[0]), pair[1]);
                }
            }
        }
    }
    let mut clusters = Vec::new();
    let mut owners = Vec::new();
    let mut cache = BTreeMap::new();
    for glyph in &frame.glyphs {
        cancel(check)?;
        let p = &frame.paragraphs[glyph.paragraph as usize];
        let paths = &p.computed.geometry.paths;
        let g = &paths.scene.as_ref().expect("scene").glyphs[glyph.glyph as usize];
        let FontFragment::Selected { shaped, .. } = &paths
            .layout
            .geometry
            .as_ref()
            .expect("geometry")
            .shaping
            .fallback
            .items[g.source.fallback_item as usize]
            .fragments[g.source.fragment as usize]
        else {
            unreachable!("complete frame")
        };
        let start = shaped.runs[0].glyphs[g.glyph as usize].cluster;
        let key = (
            glyph.paragraph,
            g.source.fallback_item,
            g.source.fragment,
            start,
        );
        if let Some(&owner) = cache.get(&key) {
            owners.push(owner);
            continue;
        }
        let end = *ends
            .get(&key)
            .ok_or(SourcePageError::Invalid("glyph cluster at fragment end"))?;
        let source = &frame.inputs[glyph.paragraph as usize].sources;
        let first = source.partition_point(|r| r.end <= start);
        let mut at = start;
        let mut runs = Vec::new();
        let mut selected = None;
        for range in &source[first..] {
            cancel(check)?;
            if range.start >= end {
                break;
            }
            if range.start > at {
                return Err(SourcePageError::Invalid("source glyph range gap"));
            }
            if range.end <= at {
                continue;
            }
            let value =
                colors(&paints[glyph.paragraph as usize][range.run as usize]).map_err(|e| {
                    e.at_run(paint::TextPaintLocation::at(
                        glyph.paragraph,
                        &frame.text.paragraphs[glyph.paragraph as usize].runs[range.run as usize],
                    ))
                })?;
            if selected.is_some_and(|previous| previous != value) {
                return Err(SourcePageError::GlyphPaintConflict {
                    paragraph: glyph.paragraph,
                    start,
                    end,
                });
            }
            selected = Some(value);
            runs.push(range.run);
            at = range.end.min(end);
        }
        if at != end {
            return Err(SourcePageError::Invalid("source glyph range incomplete"));
        }
        let color = selected.ok_or(SourcePageError::Invalid("glyph without source paint"))?;
        let owner = clusters.len() as u32;
        cache.insert(key, owner);
        owners.push(owner);
        clusters.push(GlyphClusterPaint {
            paragraph: glyph.paragraph,
            start,
            end,
            runs,
            rgba: color.fill.rgba,
            underline: color.underline.is_some(),
            underline_rgba: color.underline.and_then(|c| c.rgba),
            strike: color.strike,
        });
    }
    Ok((clusters, owners))
}
