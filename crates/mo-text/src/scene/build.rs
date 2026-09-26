use super::*;
use crate::{
    fallback::FontFragment,
    geometry::{LineGeometryResult, PreciseGlyph},
    outlines::{FontOutlinesRequest, OutlineCommand, OutlineInstance, OutlinePoint},
};
use mo_common::Emu;
use mo_font::VerifiedFont;
use mo_geometry::{BoundsBudget, Fixed, PathCommand, Point, Rect, path_bounds};
use std::collections::{BTreeMap, BTreeSet};
struct Group {
    face: usize,
    binding: u32,
    variations: Vec<ShapeVariation>,
    ids: BTreeSet<u32>,
    paths: BTreeMap<u32, Vec<OutlineCommand>>,
}
struct Placement {
    position: PreciseGlyph,
    group: usize,
    glyph_id: u32,
    size: Emu,
}
fn convert(
    commands: &[OutlineCommand],
    size: Emu,
    scale: u32,
    check: &dyn Fn() -> bool,
) -> Result<Vec<PathCommand>, TextError> {
    let point = |p: OutlinePoint| -> Result<Point, TextError> {
        Ok(Point {
            x: Fixed::scale(i64::from(p.x), size, scale)?,
            y: Fixed::ZERO.checked_sub(Fixed::scale(i64::from(p.y), size, scale)?)?,
        })
    };
    commands
        .iter()
        .map(|c| {
            cancelled(check)?;
            Ok(match *c {
                OutlineCommand::Move { to } => PathCommand::Move { to: point(to)? },
                OutlineCommand::Line { to } => PathCommand::Line { to: point(to)? },
                OutlineCommand::Quadratic { control, to } => PathCommand::Quadratic {
                    control: point(control)?,
                    to: point(to)?,
                },
                OutlineCommand::Cubic {
                    control1,
                    control2,
                    to,
                } => PathCommand::Cubic {
                    control1: point(control1)?,
                    control2: point(control2)?,
                    to: point(to)?,
                },
                OutlineCommand::Close => PathCommand::Close,
            })
        })
        .collect()
}
pub(super) struct PathInput<'a> {
    pub styles: &'a [crate::geometry::GeometryStyle],
    pub bounds_tolerance: mo_geometry::Fixed,
}
pub(super) fn build(
    q: PathInput<'_>,
    geometry: &LineGeometryResult,
    positions: Vec<PreciseGlyph>,
    fonts: &[VerifiedFont<'_>],
    bindings: &[usize],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<(Option<ParagraphPathScene>, Vec<PathSceneIssue>), TextError> {
    let mut groups: Vec<Group> = Vec::new();
    let mut registered = BTreeMap::new();
    let mut fragment_groups = BTreeMap::new();
    let mut placements = Vec::new();
    let mut unique = 0;
    for position in positions {
        cancelled(check)?;
        let item = position.source.fallback_item as usize;
        let style = geometry.shaping.items[geometry.shaping.shaped_item_indices[item] as usize]
            .style as usize;
        let FontFragment::Selected { font, shaped, .. } =
            &geometry.shaping.fallback.items[item].fragments[position.source.fragment as usize]
        else {
            unreachable!()
        };
        let run = &shaped.runs[0];
        let fragment_key = (position.source.fallback_item, position.source.fragment);
        let group = if let Some(&group) = fragment_groups.get(&fragment_key) {
            group
        } else {
            let face = bindings[*font as usize];
            let mut coords: Vec<_> = run
                .effective_variations
                .iter()
                .map(|v| (v.tag.clone(), v.effective_f32_bits))
                .collect();
            coords.sort();
            let group = if let Some(&i) = registered.get(&(face, coords.clone())) {
                i
            } else {
                if groups.len() >= 64 {
                    return Err(TextError::Limit("paragraph path font instances"));
                }
                let i = groups.len();
                registered.insert((face, coords), i);
                groups.push(Group {
                    face,
                    binding: *font,
                    variations: run
                        .effective_variations
                        .iter()
                        .map(|v| ShapeVariation {
                            tag: v.tag.clone(),
                            value_16_16: v.requested_16_16,
                        })
                        .collect(),
                    ids: BTreeSet::new(),
                    paths: BTreeMap::new(),
                });
                i
            };
            fragment_groups.insert(fragment_key, group);
            group
        };
        let glyph_id = run.glyphs[position.glyph as usize].glyph_id;
        if groups[group].ids.insert(glyph_id) {
            unique += 1;
            if unique > 4096 {
                return Err(TextError::Limit("paragraph unique outline glyphs"));
            }
        }
        placements.push(Placement {
            position,
            group,
            glyph_id,
            size: q.styles[style].font_size,
        });
    }
    let mut scene = ParagraphPathScene {
        profile: "paragraph-monochrome-paths-q32-emu-outward-bounds-v1-draft".into(),
        bounds_tolerance: q.bounds_tolerance,
        fonts: vec![],
        paths: vec![],
        glyphs: vec![],
        bounds: None,
        work: PathSceneWork {
            unique_source_glyphs: unique,
            ..Default::default()
        },
    };
    let mut issues = Vec::new();
    let mut remaining = 262144;
    for (index, group) in groups.iter_mut().enumerate() {
        let font = &fonts[group.face];
        let ids: Vec<_> = group.ids.iter().copied().collect();
        for batch in ids.chunks(256) {
            cancelled(check)?;
            let request = FontOutlinesRequest {
                expected_sha256: font.metadata().sha256.clone(),
                face_index: font.metadata().face_index,
                instances: vec![OutlineInstance {
                    variations: group.variations.clone(),
                    glyph_ids: batch.to_vec(),
                    max_commands: remaining,
                    max_operations: 1048576,
                }],
            };
            let result = outlines::extract_verified(&request, font, backend, check)?;
            scene.work.outline_calls += 1;
            if scene.fonts.len() == index {
                if !result.color_tables.is_empty() {
                    issues.push(PathSceneIssue::ColorRepresentationRequired {
                        font: group.binding,
                        variations: result.instances[0].effective_variations.clone(),
                    });
                }
                scene.fonts.push(SceneFont {
                    font_sha256: result.font_sha256,
                    face_index: result.face_index,
                    effective_variations: result.instances[0].effective_variations.clone(),
                    position_units_per_em: result.position_units_per_em,
                });
            }
            for glyph in result.instances.into_iter().next().unwrap().glyphs {
                match glyph.path {
                    Some(path) => {
                        remaining -= path.len() as u32;
                        group.paths.insert(glyph.glyph_id, path);
                    }
                    None => issues.push(PathSceneIssue::OutlineUnavailable {
                        font: group.binding,
                        variations: scene.fonts[index].effective_variations.clone(),
                        glyph_id: glyph.glyph_id,
                    }),
                };
            }
        }
    }
    if !issues.is_empty() {
        return Ok((None, issues));
    }
    let mut resources = BTreeMap::new();
    let mut budget = BoundsBudget::new(2_097_152);
    for placement in placements {
        cancelled(check)?;
        let key = (placement.group, placement.glyph_id, placement.size);
        let resource = if let Some(&r) = resources.get(&key) {
            r
        } else {
            let commands = convert(
                &groups[placement.group].paths[&placement.glyph_id],
                placement.size,
                scene.fonts[placement.group].position_units_per_em,
                check,
            )?;
            if u64::from(scene.work.path_commands) + commands.len() as u64 > 262144 {
                return Err(TextError::Limit("paragraph scaled path commands"));
            }
            scene.work.path_commands += commands.len() as u32;
            let bounds = path_bounds(&commands, q.bounds_tolerance, &mut budget, check)?;
            let r = scene.paths.len() as u32;
            resources.insert(key, r);
            scene.paths.push(GlyphPath {
                font: placement.group as u32,
                glyph_id: placement.glyph_id,
                font_size: placement.size,
                commands,
                bounds,
            });
            r
        };
        if let Some(bounds) = scene.paths[resource as usize].bounds {
            let b = bounds.translate(placement.position.origin)?;
            scene.bounds = Some(scene.bounds.map_or(b, |r: Rect| r.union(b)));
        }
        scene.glyphs.push(GlyphDraw {
            path: resource,
            origin: placement.position.origin,
            line: placement.position.line,
            source: placement.position.source,
            glyph: placement.position.glyph,
        });
    }
    scene.work.bounds_nodes = budget.visited;
    cancelled(check)?;
    Ok((Some(scene), vec![]))
}
