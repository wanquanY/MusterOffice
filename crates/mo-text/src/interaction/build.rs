use super::*;
use crate::{
    carets::*,
    fallback::FontFragment,
    geometry::{FragmentPen, PreciseFragment, PrecisePlacements},
};
use mo_font::VerifiedFont;
use std::collections::{BTreeMap, BTreeSet};

struct Cluster {
    fragment: usize,
    first: usize,
    end: usize,
    before: Point,
    after: Point,
    primary: Option<(u32, Point)>,
    group: Option<usize>,
}
struct Group {
    face: usize,
    direction: crate::Direction,
    variations: Vec<crate::ShapeVariation>,
    ids: BTreeSet<u32>,
    positions: BTreeMap<u32, Vec<i32>>,
}
fn edge(fragment: &PreciseFragment, pen: Point) -> Result<CaretEdge, TextError> {
    let baseline = fragment.origin.y.checked_sub(pen.y)?;
    Ok(CaretEdge {
        x: fragment.origin.x.checked_add(pen.x)?,
        top: baseline.checked_sub(fragment.ascent)?,
        bottom: baseline.checked_add(fragment.descent)?,
    })
}
// Round the complete rational displacement once, not a repeatedly rounded step.
fn between(a: Fixed, b: Fixed, index: usize, count: usize) -> Result<Fixed, TextError> {
    let d = b.checked_sub(a)?.raw();
    let n = index as i128;
    let den = count as i128;
    let whole = (d / den)
        .checked_mul(n)
        .ok_or(TextError::Invalid("caret interpolation range"))?;
    let remainder = (d % den) * n;
    let mut fraction = remainder / den;
    if (remainder % den).abs() * 2 >= den {
        fraction += remainder.signum();
    }
    Ok(a.checked_add(Fixed::from_raw(whole))?
        .checked_add(Fixed::from_raw(fraction))?)
}
fn partition(a: CaretEdge, b: CaretEdge, i: usize, n: usize) -> Result<CaretEdge, TextError> {
    Ok(CaretEdge {
        x: between(a.x, b.x, i, n)?,
        top: between(a.top, b.top, i, n)?,
        bottom: between(a.bottom, b.bottom, i, n)?,
    })
}
fn boundary(boundaries: &[TextBoundary], scalar: u32) -> Result<usize, TextError> {
    boundaries
        .binary_search_by_key(&scalar, |b| b.scalar_offset)
        .map_err(|_| TextError::Invalid("shaped cluster is not grapheme aligned"))
}

pub(super) struct Input<'a> {
    pub q: &'a crate::geometry::LineGeometryRequest,
    pub geometry: &'a crate::geometry::LineGeometryResult,
    pub precise: &'a PrecisePlacements,
}
pub(super) fn build(
    input: Input<'_>,
    boundaries: Vec<TextBoundary>,
    fonts: &[VerifiedFont<'_>],
    bindings: &[usize],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<InteractionMap, TextError> {
    let Input {
        q,
        geometry,
        precise,
    } = input;
    let mut groups: Vec<Group> = vec![];
    let mut keys = BTreeMap::new();
    let mut clusters = vec![];
    let mut work = InteractionWork::default();
    for (index, fragment) in precise.fragments.iter().enumerate() {
        cancelled(check)?;
        let item = &geometry.shaping.items
            [geometry.shaping.shaped_item_indices[fragment.source.fallback_item as usize] as usize];
        let FontFragment::Selected {
            start,
            end,
            font,
            shaped,
            ..
        } = &geometry.shaping.fallback.items[fragment.source.fallback_item as usize].fragments
            [fragment.source.fragment as usize]
        else {
            unreachable!()
        };
        let run = &shaped.runs[0];
        let mut ends: BTreeSet<u32> = run.glyphs.iter().map(|g| g.cluster).collect();
        ends.insert(*end);
        let logical: Vec<u32> = ends.into_iter().collect();
        let mut pen = FragmentPen::new(
            &run.glyphs,
            q.styles[item.style as usize],
            shaped.position_units_per_em,
        );
        let mut offset = 0;
        while offset < run.glyphs.len() {
            cancelled(check)?;
            let scalar = run.glyphs[offset].cluster;
            if scalar < *start || scalar >= *end {
                return Err(TextError::Invalid("caret cluster outside fragment"));
            }
            let at = logical.binary_search(&scalar).unwrap();
            // Shaping clusters can start inside an extended grapheme (for
            // example Indic syllables). Expand to the pinned EGC boundaries,
            // then join all overlapping spans without changing glyph order.
            let first = boundaries.partition_point(|b| b.scalar_offset <= scalar) - 1;
            let last = boundaries.partition_point(|b| b.scalar_offset < logical[at + 1]);
            let mut c = Cluster {
                fragment: index,
                first,
                end: last,
                before: pen.position(),
                after: pen.position(),
                primary: None,
                group: None,
            };
            let mut candidates = vec![];
            let begin = offset;
            while offset < run.glyphs.len() {
                cancelled(check)?;
                let next_scalar = run.glyphs[offset].cluster;
                let next_at = logical.binary_search(&next_scalar).unwrap();
                let lo = boundaries.partition_point(|b| b.scalar_offset <= next_scalar) - 1;
                let hi = boundaries.partition_point(|b| b.scalar_offset < logical[next_at + 1]);
                if lo >= c.end || hi <= c.first {
                    break;
                }
                c.first = c.first.min(lo);
                c.end = c.end.max(hi);
                let g = pen.advance()?.unwrap();
                let glyph = &run.glyphs[offset];
                if glyph.x_advance != 0 || glyph.y_advance != 0 {
                    candidates.push((glyph.glyph_id, g.origin));
                }
                if offset == begin {
                    c.primary = Some((glyph.glyph_id, g.origin));
                }
                c.after = g.after;
                offset += 1;
            }
            if offset - begin > 1 {
                c.primary = if candidates.len() == 1 {
                    Some(candidates[0])
                } else {
                    None
                };
            }
            if c.end - c.first > 1
                && let Some((id, _)) = c.primary
            {
                let face = bindings[*font as usize];
                let mut coords: Vec<_> = run
                    .effective_variations
                    .iter()
                    .map(|v| (v.tag.clone(), v.effective_f32_bits))
                    .collect();
                coords.sort();
                let key = (face, run.direction.word(), coords);
                let group = if let Some(&g) = keys.get(&key) {
                    g
                } else {
                    if groups.len() >= 64 {
                        return Err(TextError::Limit("interaction font instances"));
                    }
                    let g = groups.len();
                    keys.insert(key, g);
                    groups.push(Group {
                        face,
                        direction: run.direction,
                        variations: run
                            .effective_variations
                            .iter()
                            .map(|v| crate::ShapeVariation {
                                tag: v.tag.clone(),
                                value_16_16: v.requested_16_16,
                            })
                            .collect(),
                        ids: BTreeSet::new(),
                        positions: BTreeMap::new(),
                    });
                    g
                };
                if groups[group].ids.insert(id) {
                    work.unique_glyphs += 1;
                    if work.unique_glyphs > 4096 {
                        return Err(TextError::Limit("interaction unique glyphs"));
                    }
                }
                c.group = Some(group);
            }
            clusters.push(c);
        }
    }
    work.font_instances = groups.len() as u32;
    for group in &mut groups {
        let font = &fonts[group.face];
        let ids: Vec<_> = group.ids.iter().copied().collect();
        for batch in ids.chunks(256) {
            cancelled(check)?;
            let result = crate::carets::query_verified(
                &FontCaretsRequest {
                    expected_sha256: font.metadata().sha256.clone(),
                    face_index: font.metadata().face_index,
                    instances: vec![FontCaretsInstance {
                        variations: group.variations.clone(),
                        direction: group.direction,
                        glyph_ids: batch.to_vec(),
                    }],
                },
                font,
                backend,
                check,
            )?;
            work.caret_calls += 1;
            for glyph in result.instances.into_iter().next().unwrap().glyphs {
                group.positions.insert(glyph.glyph_id, glyph.positions);
            }
        }
    }
    let mut cells: Vec<Option<InteractionCell>> = vec![None; boundaries.len() - 1];
    for cluster in clusters {
        cancelled(check)?;
        let fragment = &precise.fragments[cluster.fragment];
        let item = &geometry.shaping.items
            [geometry.shaping.shaped_item_indices[fragment.source.fallback_item as usize] as usize];
        let FontFragment::Selected { shaped, .. } = &geometry.shaping.fallback.items
            [fragment.source.fallback_item as usize]
            .fragments[fragment.source.fragment as usize]
        else {
            unreachable!()
        };
        let n = cluster.end - cluster.first;
        let before = edge(fragment, cluster.before)?;
        let after = edge(fragment, cluster.after)?;
        let (mut stops, placement) = if n == 1 {
            (vec![before, after], CaretPlacement::GlyphEdges)
        } else {
            let (positions, reason) = match (cluster.group, cluster.primary) {
                (Some(g), Some((id, _))) => {
                    let values = &groups[g].positions[&id];
                    if values.is_empty() {
                        (None, PartitionReason::FontCaretsAbsent)
                    } else if values.len() != n - 1 {
                        (None, PartitionReason::FontCaretCountMismatch)
                    } else if values.windows(2).any(|p| p[0] > p[1]) {
                        (None, PartitionReason::NonMonotoneFontCarets)
                    } else {
                        (Some(values), PartitionReason::FontCaretsAbsent)
                    }
                }
                _ => (None, PartitionReason::AmbiguousGlyphs),
            };
            if let Some(values) = positions {
                let (_, origin) = cluster.primary.unwrap();
                let mut stops = vec![before];
                for &value in values {
                    let mut e = edge(fragment, origin)?;
                    e.x = e.x.checked_add(Fixed::scale(
                        i64::from(value),
                        q.styles[item.style as usize].font_size,
                        shaped.position_units_per_em,
                    )?)?;
                    stops.push(e);
                }
                stops.push(after);
                (stops, CaretPlacement::FontLigature)
            } else {
                let stops = (0..=n)
                    .map(|i| partition(before, after, i, n))
                    .collect::<Result<Vec<_>, _>>()?;
                (stops, CaretPlacement::ClusterPartition { reason })
            }
        };
        if item.level % 2 == 1 {
            stops.reverse();
        }
        for i in 0..n {
            let at = cluster.first + i;
            if cells[at].is_some() {
                return Err(TextError::Invalid("overlapping interaction clusters"));
            }
            cells[at] = Some(InteractionCell {
                start: boundaries[at].clone(),
                end: boundaries[at + 1].clone(),
                line: fragment.line,
                level: item.level,
                kind: item.kind,
                leading: stops[i],
                trailing: stops[i + 1],
                placement,
            });
        }
    }
    let layout = precise
        .layout
        .as_ref()
        .ok_or(TextError::Invalid("interaction needs exact layout"))?;
    let mut lines = vec![];
    for (index, line) in geometry.shaping.lines.iter().enumerate() {
        cancelled(check)?;
        let first = boundary(&boundaries, line.start.scalar_offset)?;
        let end = boundary(&boundaries, line.end.scalar_offset)?;
        let box_ = &layout.lines[index];
        let (top, bottom) = precise.empty_line_carets[index];
        let empty = CaretEdge {
            x: Fixed::ZERO,
            top,
            bottom,
        };
        // Removed controls and text with no glyphs get a stable logical neighbor
        // edge; one reverse and one forward pass avoid quadratic control runs.
        let mut next = None;
        let mut following = vec![None; end - first];
        for at in (first..end).rev() {
            following[at - first] = next;
            if let Some(c) = &cells[at] {
                next = Some(c.leading);
            }
        }
        let mut previous = None;
        for at in first..end {
            cancelled(check)?;
            if cells[at].is_none() {
                let scalar = boundaries[at].scalar_offset;
                let item_index = geometry
                    .shaping
                    .items
                    .partition_point(|item| item.end.scalar_offset <= scalar);
                let item = &geometry.shaping.items[item_index];
                let e = previous.or(following[at - first]).unwrap_or(empty);
                cells[at] = Some(InteractionCell {
                    start: boundaries[at].clone(),
                    end: boundaries[at + 1].clone(),
                    line: index as u32,
                    level: item.level,
                    kind: item.kind,
                    leading: e,
                    trailing: e,
                    placement: CaretPlacement::Invisible,
                });
            }
            previous = Some(cells[at].as_ref().unwrap().trailing);
        }
        lines.push(InteractionLine {
            start: line.start.clone(),
            end: line.end.clone(),
            top: box_.top,
            bottom: box_.bottom,
            empty_caret: empty,
            cells: (first as u32..end as u32).collect(),
        });
    }
    let cells = cells
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(TextError::Invalid("incomplete interaction coverage"))?;
    cancelled(check)?;
    Ok(InteractionMap {
        profile: "unicode18-hb14.5-horizontal-interaction-q32-v1-draft".into(),
        paragraph_level: geometry.shaping.bidi.paragraph_level,
        boundaries,
        lines,
        cells,
        work,
    })
}
