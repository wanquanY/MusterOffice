//! Advance-based, font-metric single underlines and strikes. Glyph ink bounds
//! and combining-mark offsets are deliberately not decoration extents.
use super::*;
use mo_geometry::{PathCommand as C, Rect};
use mo_text::{
    ShapeVariation,
    fallback::FontFragment,
    metrics::{FontMetric as M, FontMetricsInstance, MeasuredInstance},
};
type Key = (u32, Vec<(String, i32)>);
struct Instance {
    key: Key,
    underline: bool,
    strike: bool,
    scale: u32,
    measured: Option<MeasuredInstance>,
}
fn key(font: u32, run: &mo_text::ShapedRun) -> Key {
    let mut axes: Vec<_> = run
        .effective_variations
        .iter()
        .map(|v| (v.tag.clone(), v.requested_16_16))
        .collect();
    axes.sort();
    (font, axes)
}
pub(super) fn build(
    frame: &SourceFramePlan,
    clusters: &[GlyphClusterPaint],
    owners: &[u32],
    manifest: &PreparedManifest<'_, '_>,
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<Vec<TextDecoration>, SourcePageError> {
    let mut instances: Vec<Instance> = Vec::new();
    let mut unique = BTreeMap::new();
    let mut glyph_instances = Vec::with_capacity(owners.len());
    for (g, owner) in frame.glyphs.iter().zip(owners) {
        cancel(check)?;
        let c = &clusters[*owner as usize];
        if c.underline_rgba.is_none() && (!c.strike || c.rgba.is_none()) {
            glyph_instances.push(None);
            continue;
        }
        let paths = &frame.paragraphs[g.paragraph as usize]
            .computed
            .geometry
            .paths;
        let draw = &paths.scene.as_ref().expect("scene").glyphs[g.glyph as usize];
        let FontFragment::Selected { font, shaped, .. } = &paths
            .layout
            .geometry
            .as_ref()
            .expect("geometry")
            .shaping
            .fallback
            .items[draw.source.fallback_item as usize]
            .fragments[draw.source.fragment as usize]
        else {
            unreachable!("complete frame")
        };
        let k = key(*font, &shaped.runs[0]);
        let index = if let Some(&i) = unique.get(&k) {
            i
        } else {
            if instances.len() >= 2048 {
                return Err(RasterError::Limit("decoration instances").into());
            }
            let i = instances.len();
            unique.insert(k.clone(), i);
            instances.push(Instance {
                key: k,
                underline: false,
                strike: false,
                scale: 0,
                measured: None,
            });
            i
        };
        instances[index].underline |= c.underline_rgba.is_some();
        instances[index].strike |= c.strike && c.rgba.is_some();
        glyph_instances.push(Some(index));
    }
    if instances.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_font = BTreeMap::<u32, Vec<usize>>::new();
    for (i, instance) in instances.iter().enumerate() {
        by_font.entry(instance.key.0).or_default().push(i);
    }
    for (font, indices) in by_font {
        for batch in indices.chunks(256) {
            cancel(check)?;
            let requests: Vec<_> = batch
                .iter()
                .map(|&i| {
                    let instance = &instances[i];
                    let mut metrics = Vec::new();
                    if instance.underline {
                        metrics.extend([M::UnderlineOffset, M::UnderlineSize]);
                    }
                    if instance.strike {
                        metrics.extend([M::StrikeoutOffset, M::StrikeoutSize]);
                    }
                    FontMetricsInstance {
                        variations: instance
                            .key
                            .1
                            .iter()
                            .map(|(tag, value)| ShapeVariation {
                                tag: tag.clone(),
                                value_16_16: *value,
                            })
                            .collect(),
                        metrics,
                    }
                })
                .collect();
            let result = manifest
                .measure_instances(font, &requests, backend, check)
                .map_err(SourceFrameError::from)?;
            for (&i, value) in batch.iter().zip(result.instances) {
                instances[i].scale = result.position_units_per_em;
                instances[i].measured = Some(value);
            }
        }
    }
    // Reconstruct a fragment's pen origin once, then use exact design-unit
    // prefix sums. Never accumulate individually rounded glyph advances.
    let mut pens = BTreeMap::<(u32, u32, u32), (Point, mo_text::geometry::FragmentPen<'_>)>::new();
    let mut result: Vec<TextDecoration> = Vec::new();
    let mut last = [None::<usize>; 2];
    for ((g, owner), instance) in frame.glyphs.iter().zip(owners).zip(glyph_instances) {
        cancel(check)?;
        let p = &frame.paragraphs[g.paragraph as usize];
        let paths = &p.computed.geometry.paths;
        let scene = paths.scene.as_ref().expect("scene");
        let draw = &scene.glyphs[g.glyph as usize];
        let path = &scene.paths[draw.path as usize];
        let FontFragment::Selected { shaped, .. } = &paths
            .layout
            .geometry
            .as_ref()
            .expect("geometry")
            .shaping
            .fallback
            .items[draw.source.fallback_item as usize]
            .fragments[draw.source.fragment as usize]
        else {
            unreachable!("complete frame")
        };
        let geometry = paths.layout.geometry.as_ref().expect("geometry");
        let style = frame.inputs[g.paragraph as usize].geometry[geometry.shaping.items
            [geometry.shaping.shaped_item_indices[draw.source.fallback_item as usize] as usize]
            .style as usize];
        let fragment = (g.paragraph, draw.source.fallback_item, draw.source.fragment);
        let pen = if let Some(pen) = pens.get_mut(&fragment) {
            pen
        } else {
            if draw.glyph != 0 {
                return Err(SourcePageError::Invalid("decoration fragment order"));
            }
            let glyph = &shaped.runs[0].glyphs[0];
            let scale = |raw| Fixed::scale(raw, path.font_size, shaped.position_units_per_em);
            pens.entry(fragment).or_insert((
                Point {
                    x: g.origin.x.checked_sub(scale(i64::from(glyph.x_offset))?)?,
                    y: g.origin.y.checked_add(scale(i64::from(glyph.y_offset))?)?,
                },
                mo_text::geometry::FragmentPen::new(
                    &shaped.runs[0].glyphs,
                    style,
                    shaped.position_units_per_em,
                ),
            ))
        };
        let position = pen
            .1
            .advance()
            .map_err(SourceFrameError::from)?
            .ok_or(SourcePageError::Invalid("decoration glyph order"))?;
        if position.glyph != draw.glyph {
            return Err(SourcePageError::Invalid("decoration glyph order"));
        }
        let x0 = pen.0.x.checked_add(position.before.x)?;
        let x1 = pen.0.x.checked_add(position.after.x)?;
        let baseline = pen.0.y.checked_sub(position.before.y)?;
        let Some(instance) = instance else {
            last = [None; 2];
            continue;
        };
        let c = &clusters[*owner as usize];
        let instance = &instances[instance];
        let metric = |m: M, thickness: bool| -> Result<Fixed, SourcePageError> {
            let value = instance
                .measured
                .as_ref()
                .expect("measured")
                .values
                .iter()
                .find(|v| v.metric == m)
                .and_then(|v| v.position);
            if value.is_none() || (thickness && value.is_some_and(|v| v <= 0)) {
                return Err(SourcePageError::TextDecoration(Box::new(
                    TextDecorationIssue {
                        paragraph: frame.text.paragraph_start + g.paragraph,
                        run: c.runs[0],
                        source_ordinal: frame.text.paragraphs[g.paragraph as usize].runs
                            [c.runs[0] as usize]
                            .source_ordinal,
                        font: instance.key.0,
                        metric: m,
                        value,
                    },
                )));
            }
            Ok(Fixed::scale(
                i64::from(value.unwrap()),
                path.font_size,
                instance.scale,
            )?)
        };
        for (slot, (rgba, kind, offset, size)) in [
            (
                c.underline_rgba,
                DecorationKind::Underline,
                M::UnderlineOffset,
                M::UnderlineSize,
            ),
            (
                if c.strike { c.rgba } else { None },
                DecorationKind::Strike,
                M::StrikeoutOffset,
                M::StrikeoutSize,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let Some(rgba) = rgba else {
                last[slot] = None;
                continue;
            };
            // Both OpenType post and OS/2 positions specify the TOP in y-up.
            let top = baseline.checked_sub(metric(offset, false)?)?;
            let bottom = top.checked_add(metric(size, true)?)?;
            if x0 == x1 {
                continue;
            }
            let rect = Rect {
                min: Point {
                    x: x0.min(x1),
                    y: top,
                },
                max: Point {
                    x: x0.max(x1),
                    y: bottom,
                },
            };
            if let Some(previous) = last[slot].and_then(|i| result.get_mut(i))
                && previous.paragraph == g.paragraph
                && previous.line == draw.line
                && previous.rgba == rgba
                && previous.rect.min.y == top
                && previous.rect.max.y == bottom
                && rect.min.x <= previous.rect.max.x.checked_add(Fixed::from_raw(2))?
                && rect.max.x >= previous.rect.min.x.checked_sub(Fixed::from_raw(2))?
            {
                previous.rect = previous.rect.union(rect);
                if previous.clusters.last() != Some(owner) {
                    previous.clusters.push(*owner);
                }
                continue;
            }
            last[slot] = Some(result.len());
            result.push(TextDecoration {
                paragraph: g.paragraph,
                line: draw.line,
                kind,
                clusters: vec![*owner],
                rect,
                rgba,
            });
        }
    }
    Ok(result)
}
pub(super) fn commands(rect: Rect) -> [C; 5] {
    [
        C::Move { to: rect.min },
        C::Line {
            to: Point {
                x: rect.max.x,
                y: rect.min.y,
            },
        },
        C::Line { to: rect.max },
        C::Line {
            to: Point {
                x: rect.min.x,
                y: rect.max.y,
            },
        },
        C::Close,
    ]
}
