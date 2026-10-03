use super::*;
use mo_common::Emu;
use mo_text::{
    geometry::{GeometryStyle, LineSpacing},
    itemize::StyleSpan,
    manifest::*,
};
fn shaping(style: &style::TextStyle) -> ManifestTextStyle {
    ManifestTextStyle {
        typeface: style.family.clone(),
        font_style: match (style.bold, style.italic) {
            (false, false) => FontStyle::Regular,
            (true, false) => FontStyle::Bold,
            (false, true) => FontStyle::Italic,
            (true, true) => FontStyle::BoldItalic,
        },
        language: style.language.clone(),
        features: vec![],
        suppress_dotted_circle: false,
        max_glyphs: 65536,
    }
}
fn input<'a>(
    label: &'a Label,
    styles: &'a [ManifestTextStyle],
    spans: &'a [StyleSpan],
) -> ManifestParagraphInput<'a> {
    ManifestParagraphInput {
        text: &label.text,
        direction: mo_unicode::bidi::ParagraphDirection::AutoLeftToRight,
        spans,
        styles,
    }
}
pub(super) fn validate(
    labels: &[Label],
    context: &TextPageContext<'_, '_, '_>,
    check: &dyn Fn() -> bool,
) -> Result<(), SourcePageError> {
    for label in labels {
        cancel(check)?;
        if label.text.len() > 16384 || !label.width.is_finite() || label.width <= 0.0 {
            return Err(RasterError::Limit("chart label content/width").into());
        }
        context
            .manifest
            .validate_paragraph(
                input(
                    label,
                    &[shaping(&label.style)],
                    &[StyleSpan {
                        end: label.text.chars().count() as u32,
                        style: 0,
                    }],
                ),
                check,
            )
            .map_err(crate::source_frame::SourceFrameError::from)?;
    }
    Ok(())
}
pub(crate) fn shape(
    chart: &mut ChartPage,
    manifest: &PreparedManifest<'_, '_>,
    backend: &mut crate::source_frame::backend::FrameBackend<'_>,
    path_bytes: &mut u64,
    check: &dyn Fn() -> bool,
) -> Result<(), SourcePageError> {
    let before = backend.work.clone();
    for label in &chart.labels {
        cancel(check)?;
        if label.text.is_empty() {
            continue;
        }
        let styles = [shaping(&label.style)];
        let spans = [StyleSpan {
            end: label.text.chars().count() as u32,
            style: 0,
        }];
        let geometry = [GeometryStyle {
            font_size: Emu::new(label.style.size.round() as i64),
            baseline_shift: Emu::new(0).into(),
            cluster_spacing: Fixed::ZERO,
        }];
        let paths = manifest
            .paragraph_paths(
                ManifestLayoutInput {
                    paragraph: input(label, &styles, &spans),
                    styles: &geometry,
                    strut_style: 0,
                    spacing: LineSpacing::Natural,
                    width: Emu::new(label.width.round() as i64),
                    overflow: mo_text::flow::OverflowPolicy::KeepUnbreakable,
                },
                Fixed::from_raw(1 << 24),
                backend,
                check,
            )
            .map_err(crate::source_frame::SourceFrameError::from)?;
        let geometry = paths
            .paths
            .layout
            .geometry
            .as_ref()
            .and_then(|g| g.layout.as_ref())
            .ok_or(SourcePageError::Invalid("chart text layout unresolved"))?;
        if geometry.lines.len() != 1 || geometry.lines[0].pen_max.get() as f64 > label.width + 1.0 {
            return Err(SourcePageError::Invalid(
                "chart label does not fit its layout region",
            ));
        }
        let line = &geometry.lines[0];
        let width = (line.pen_max.get() - line.pen_min.get()) as f64;
        let x = label.x + (label.width - width) * label.align - line.pen_min.get() as f64;
        let y = label.y - geometry.height.get() as f64 / 2.0;
        let scene = paths
            .paths
            .scene
            .ok_or(SourcePageError::Invalid("chart glyph outlines unavailable"))?;
        backend.work.glyphs = backend
            .work
            .glyphs
            .checked_add(scene.glyphs.len() as u32)
            .ok_or(RasterError::Range)?;
        backend.work.path_commands = backend
            .work
            .path_commands
            .checked_add(scene.work.path_commands)
            .ok_or(RasterError::Range)?;
        if backend.work.glyphs > backend.limits.max_glyphs
            || backend.work.path_commands > backend.limits.max_path_commands
        {
            return Err(RasterError::Limit("chart text paths").into());
        }
        for glyph in scene.glyphs {
            let path = &scene.paths[glyph.path as usize];
            if path.commands.is_empty() {
                continue;
            }
            charge_paths(path_bytes, path.commands.len())?;
            chart.info.path_bytes += 160 + path.commands.len() as u64 * 112;
            chart.draws.push(Draw {
                ordinal: label.ordinal,
                commands: path.commands.clone(),
                origin: Point {
                    x: glyph.origin.x.checked_add(fixed(x)?)?,
                    y: glyph.origin.y.checked_add(fixed(y)?)?,
                },
                rgba: label.style.rgba,
                stroke: None,
                error: Fixed::emu(Emu::new(2)),
            });
        }
    }
    chart.info.text_work = crate::source_frame::FrameWork {
        component_calls: backend.work.component_calls - before.component_calls,
        font_upload_bytes: backend.work.font_upload_bytes - before.font_upload_bytes,
        request_words: backend.work.request_words - before.request_words,
        glyphs: backend.work.glyphs - before.glyphs,
        path_commands: backend.work.path_commands - before.path_commands,
    };
    chart.info.paths = chart
        .draws
        .len()
        .try_into()
        .map_err(|_| RasterError::Range)?;
    chart.labels.clear();
    Ok(())
}
