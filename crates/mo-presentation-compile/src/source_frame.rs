//! Source-native horizontal frame flow and unpainted glyph placement. This is
//! shape-local geometry; page transforms, paint, clipping and publication remain
//! the page compiler's responsibility.
pub(crate) mod backend;
mod number;
mod properties;
mod spacing;
mod types;
use crate::source_text::{self, SourceParagraphPlan, SourceTextLimits, SourceTextPreparation};
use mo_geometry::{Fixed, Point, Rect};
use mo_pptx::source::{
    SourceIndex,
    geometry::evaluate::{self, *},
    text::{
        NativeTextAlign, NativeTextAnchor,
        body::{self, *},
    },
};
use mo_text::{
    backend::TextBackend,
    manifest::{ManifestFlowInput, ManifestParagraphInput, PreparedManifest},
};
pub use types::*;
pub const PROFILE: &str = "drawingml-horizontal-source-frame-q32-draft-v1";
fn mapping(issue: SourceFrameIssue) -> SourceFrameError {
    SourceFrameError::Mapping(Box::new(issue))
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourceFrameError> {
    if check() {
        Err(SourceFrameError::Cancelled)
    } else {
        Ok(())
    }
}
fn input(p: &SourceParagraphPlan) -> ManifestParagraphInput<'_> {
    ManifestParagraphInput {
        text: &p.text,
        direction: p.direction,
        spans: &p.spans,
        styles: &p.styles,
    }
}

pub fn compile(
    index: &SourceIndex,
    q: &SourceFrameRequest,
    manifest: &PreparedManifest<'_, '_>,
    backend: &mut dyn TextBackend,
    limits: SourceFrameLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFramePlan, SourceFrameError> {
    let prepared = prepare(index, q, manifest, limits, check)?;
    compute(prepared, manifest, backend, limits, check)
}
pub(crate) struct PreparedFrame {
    prepared: crate::source_text::PreparedSourceText,
    body: EffectiveTextBody,
    region: SourceFrameRegion,
    specs: Vec<FrameParagraphSpec>,
    bounds_tolerance: Fixed,
}
impl PreparedFrame {
    pub(crate) fn accounted_plan_bytes(&self) -> usize {
        self.prepared.accounted_plan_bytes()
    }
    pub(crate) fn source(&self) -> &mo_pptx::source::text::cascade::CascadedText {
        self.prepared.source()
    }
}
pub(crate) fn prepare(
    index: &SourceIndex,
    q: &SourceFrameRequest,
    manifest: &PreparedManifest<'_, '_>,
    limits: SourceFrameLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreparedFrame, SourceFrameError> {
    cancel(check)?;
    if !(256..=1i128 << 32).contains(&q.bounds_tolerance.raw()) {
        return Err(mo_text::TextError::Invalid("path bounds tolerance").into());
    }
    let prepared = match source_text::prepare(
        index,
        &q.expected_source_sha256,
        &q.object,
        SourceTextLimits::default(),
        check,
    )? {
        SourceTextPreparation::Prepared { text } => text,
        SourceTextPreparation::Unresolved { issue } => {
            return Err(mapping(SourceFrameIssue::Text { reason: issue }));
        }
    };
    if prepared.paragraphs().len() > limits.max_paragraphs {
        return Err(SourceFrameError::Limit("frame paragraphs"));
    }
    let mut body = body::query(
        index,
        &SourceTextBodyQuery {
            expected_source_sha256: q.expected_source_sha256.clone(),
            surface: q.object.part.clone(),
            objects: vec![q.object.native_id],
            profile: TextBodyProfile::DrawingmlDraftV1,
        },
        TextBodyLimits::default(),
        check,
    )?;
    let body = match body.objects.remove(0).outcome {
        TextBodyOutcome::Resolved { body } => *body,
        TextBodyOutcome::Unresolved { reason } => {
            return Err(mapping(SourceFrameIssue::Body { reason }));
        }
    };
    properties::body(&body)?;
    let mut geometry = evaluate::query(
        index,
        &SourceGeometryQuery {
            expected_source_sha256: q.expected_source_sha256.clone(),
            surface: q.object.part.clone(),
            objects: vec![q.object.native_id],
            profile: GeometryProfile::Drawingml2016PresetsDraftV2,
        },
        GeometryLimits::default(),
        check,
    )?;
    let geometry = match geometry.objects.remove(0).outcome {
        GeometryOutcome::Resolved { geometry } => geometry,
        GeometryOutcome::Unresolved { reason } => {
            return Err(mapping(SourceFrameIssue::Geometry { reason }));
        }
    };
    let region = number::region(&geometry, &body)?;
    let mut specs = Vec::new();
    for (i, (native, p)) in prepared
        .source()
        .paragraphs
        .iter()
        .zip(prepared.paragraphs())
        .enumerate()
    {
        cancel(check)?;
        specs.push(properties::paragraph(
            index, i as u32, native, p, &region, check,
        )?);
        manifest
            .validate_paragraph(input(p), check)
            .map_err(|e| prepared.text_error(i as u32, e))?;
    }
    cancel(check)?;
    Ok(PreparedFrame {
        prepared,
        body,
        region,
        specs,
        bounds_tolerance: q.bounds_tolerance,
    })
}
pub(crate) fn compute(
    frame: PreparedFrame,
    manifest: &PreparedManifest<'_, '_>,
    backend: &mut dyn TextBackend,
    limits: SourceFrameLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFramePlan, SourceFrameError> {
    let PreparedFrame {
        prepared,
        body,
        region,
        specs,
        bounds_tolerance,
    } = frame;
    let mut worker = backend::FrameBackend {
        inner: backend,
        work: FrameWork::default(),
        limits,
    };
    let mut paragraphs = Vec::new();
    let mut y = Fixed::ZERO;
    for (i, (p, spec)) in prepared.paragraphs().iter().zip(specs).enumerate() {
        cancel(check)?;
        let computed = manifest
            .paragraph_geometry(
                ManifestFlowInput {
                    paragraph: input(p),
                    styles: &p.geometry,
                    strut_style: p.end_style,
                    spacing: spec.spacing.clone(),
                    widths: spec.widths,
                    overflow: spec.overflow,
                },
                bounds_tolerance,
                &mut worker,
                check,
            )
            .map_err(|e| prepared.text_error(i as u32, e))?;
        let incomplete = || {
            mapping(SourceFrameIssue::IncompleteParagraph {
                paragraph: i as u32,
                flow: computed.geometry.paths.layout.issues.clone(),
                geometry: computed
                    .geometry
                    .paths
                    .layout
                    .geometry
                    .as_ref()
                    .map_or_else(Vec::new, |g| g.issues.clone()),
                paths: computed.geometry.paths.issues.clone(),
            })
        };
        let scene = computed
            .geometry
            .paths
            .scene
            .as_ref()
            .ok_or_else(incomplete)?;
        let precise = computed.geometry.precise.as_ref().ok_or_else(incomplete)?;
        worker.work.glyphs = worker
            .work
            .glyphs
            .checked_add(scene.glyphs.len() as u32)
            .ok_or(SourceFrameError::Limit("frame glyphs"))?;
        worker.work.path_commands = worker
            .work
            .path_commands
            .checked_add(scene.work.path_commands)
            .ok_or(SourceFrameError::Limit("frame path commands"))?;
        if worker.work.glyphs > limits.max_glyphs
            || worker.work.path_commands > limits.max_path_commands
        {
            return Err(SourceFrameError::Limit("frame glyph/path result"));
        }
        let shaping = &computed
            .geometry
            .paths
            .layout
            .geometry
            .as_ref()
            .expect("complete geometry")
            .shaping;
        let before = if i > 0 || body.attributes.paragraph_spacing == Some(true) {
            spacing::resolve(&spec.before, shaping, 0, p.end_style)?
        } else {
            Fixed::ZERO
        };
        let after = if i + 1 < prepared.paragraphs().len()
            || body.attributes.paragraph_spacing == Some(true)
        {
            spacing::resolve(
                &spec.after,
                shaping,
                (precise.lines.len() - 1) as u32,
                p.end_style,
            )?
        } else {
            Fixed::ZERO
        };
        y = y.checked_add(before)?;
        let mut line_offsets = Vec::new();
        for (line, g) in precise.lines.iter().enumerate() {
            cancel(check)?;
            let width = if line == 0 {
                spec.widths.first
            } else {
                spec.widths.rest
            };
            let indent = if line == 0 && !spec.indent_from_right {
                spec.indent
            } else {
                Fixed::ZERO
            };
            let x = region
                .inner
                .min
                .x
                .checked_add(spec.left)?
                .checked_add(indent)?;
            let offset = match spec.alignment {
                NativeTextAlign::L => Fixed::ZERO,
                NativeTextAlign::R => width.checked_sub(g.pen_max)?,
                NativeTextAlign::Ctr => width
                    .checked_sub(g.pen_max)?
                    .checked_sub(g.pen_min)?
                    .half()?,
                _ => unreachable!("preflight alignment"),
            };
            line_offsets.push(Point {
                x: x.checked_add(offset)?,
                y,
            });
        }
        y = y.checked_add(precise.height)?;
        y = y.checked_add(after)?;
        paragraphs.push(FrameParagraph {
            spec,
            computed,
            line_offsets,
            applied_before: before,
            applied_after: after,
        });
    }
    let height = region.inner.max.y.checked_sub(region.inner.min.y)?;
    let vertical = match body.attributes.anchor.expect("preflight anchor") {
        NativeTextAnchor::T => Fixed::ZERO,
        NativeTextAnchor::Ctr => height.checked_sub(y)?.half()?,
        NativeTextAnchor::B => height.checked_sub(y)?,
        _ => unreachable!("preflight anchor"),
    }
    .checked_add(region.inner.min.y)?;
    let mut glyphs = Vec::new();
    let mut bounds: Option<Rect> = None;
    for (p, paragraph) in paragraphs.iter_mut().enumerate() {
        for line in &mut paragraph.line_offsets {
            line.y = line.y.checked_add(vertical)?;
        }
        let scene = paragraph
            .computed
            .geometry
            .paths
            .scene
            .as_ref()
            .expect("complete paragraph");
        for (i, glyph) in scene.glyphs.iter().enumerate() {
            cancel(check)?;
            let origin = glyph
                .origin
                .translate(paragraph.line_offsets[glyph.line as usize])?;
            if let Some(local) = scene.paths[glyph.path as usize].bounds {
                let value = local.translate(origin)?;
                bounds = Some(bounds.map_or(value, |b| b.union(value)));
            }
            glyphs.push(FrameGlyph {
                paragraph: p as u32,
                glyph: i as u32,
                origin,
            });
        }
    }
    cancel(check)?;
    let (text, inputs) = prepared.into_parts();
    let align = if body.attributes.anchor == Some(NativeTextAnchor::Ctr)
        || paragraphs
            .iter()
            .any(|p| p.spec.alignment == NativeTextAlign::Ctr)
    {
        Fixed::from_raw(1)
    } else {
        Fixed::ZERO
    };
    Ok(SourceFramePlan {
        profile: PROFILE.into(),
        text,
        inputs,
        body,
        region,
        paragraphs,
        content_height: y,
        glyphs,
        bounds,
        alignment_rounding_bound: align,
        work: worker.work,
    })
}
