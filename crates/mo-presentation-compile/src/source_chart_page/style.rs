use super::*;
use mo_presentation_source::source::{
    charts::paints::{ChartPaintDeclaration, SourceChartPaints},
    color::ColorSample,
    fill::SourceFillDefinition,
    line::{NativeLineCap, NativePresetDash, SourceLineDash, SourceLineFill, SourceLineJoin},
    text::{NativeTextElement as N, SourceTextValue as V},
};

#[derive(Clone)]
pub(super) struct TextStyle {
    pub family: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub language: String,
    pub rgba: [u8; 4],
}
pub(super) struct Paint {
    pub fill: Option<[u8; 4]>,
    pub line: Option<([u8; 4], StrokeStyle)>,
}
pub(super) fn value(layout: &SourceChartLayout, kind: ChartPropertyKind) -> Option<&str> {
    layout
        .properties
        .iter()
        .find(|p| p.kind == kind)
        .and_then(|p| p.value.as_deref())
}
pub(super) fn number(
    layout: &SourceChartLayout,
    kind: ChartPropertyKind,
    default: f64,
    part: &str,
) -> Result<f64, SourcePageError> {
    let v = match value(layout, kind) {
        Some(v) => v
            .parse()
            .map_err(|_| invalid(part, 0, "invalid chart numeric property"))?,
        None => default,
    };
    if !f64::is_finite(v) || v.abs() > 1e12 {
        return Err(invalid(part, 0, "chart numeric property out of range"));
    }
    Ok(v)
}
pub(super) fn on(layout: &SourceChartLayout, kind: ChartPropertyKind) -> bool {
    matches!(value(layout, kind), Some("1" | "true"))
}
pub(super) fn audit(
    layout: &SourceChartLayout,
    allowed: &[ChartPropertyKind],
    markup: &[ChartMarkupKind],
    part: &str,
) -> Result<(), SourcePageError> {
    if let Some(v) = layout.unrecognized_children.first() {
        return Err(invalid(
            part,
            v.source_ordinal,
            format!("unsupported chart element {}", v.local_name),
        ));
    }
    if let Some(v) = layout.retained_attribute_ordinals.first() {
        return Err(invalid(part, *v, "unsupported chart attribute"));
    }
    for p in &layout.properties {
        if !allowed.contains(&p.kind) || p.value.is_none() {
            return Err(invalid(
                part,
                p.source_ordinal,
                format!("unsupported chart property {:?}", p.kind),
            ));
        }
    }
    for m in &layout.markup {
        if !markup.contains(&m.kind) {
            return Err(invalid(
                part,
                m.source_ordinal,
                format!("unsupported chart markup {:?}", m.kind),
            ));
        }
    }
    if let Some(m) = &layout.marker
        && (m.symbol.as_deref() != Some("none") || !m.retained_ordinals.is_empty())
    {
        return Err(invalid(
            part,
            m.source_ordinal,
            "chart marker requires supported native symbol",
        ));
    }
    Ok(())
}
pub(super) fn color(source: &SourceChartPaints, ordinal: u32) -> Result<[u8; 4], SourcePageError> {
    let colors = source
        .declarations
        .iter()
        .flat_map(|p| &p.colors)
        .chain(&source.text_colors);
    match colors
        .into_iter()
        .find(|c| c.source_ordinal == ordinal)
        .map(|c| &c.outcome)
    {
        Some(ColorSample::Resolved { rgba8, .. }) => Ok(*rgba8),
        _ => Err(invalid(
            &source.chart.part,
            ordinal,
            "chart color context unresolved",
        )),
    }
}
pub(super) fn declaration(
    source: &SourceChartPaints,
    parent: u32,
) -> Option<&ChartPaintDeclaration> {
    source
        .declarations
        .iter()
        .find(|d| d.parent_ordinal == Some(parent))
}
pub(super) fn paint(source: &SourceChartPaints, parent: u32) -> Result<Paint, SourcePageError> {
    let Some(d) = declaration(source, parent) else {
        return Ok(Paint {
            fill: None,
            line: None,
        });
    };
    let fail = |s| invalid(&source.chart.part, d.source_ordinal, s);
    if !d.retained_ordinals.is_empty()
        || d.black_white_mode.is_some()
        || d.effects
            .as_ref()
            .is_some_and(|e| !e.is_explicitly_empty_list())
    {
        return Err(fail("unsupported chart paint/effect"));
    }
    let fill = match d.fill.as_ref().map(|f| &f.definition) {
        None | Some(SourceFillDefinition::None {}) => None,
        Some(SourceFillDefinition::Solid { color: Some(c) }) => {
            Some(color(source, c.source_ordinal)?)
        }
        _ => return Err(fail("chart fill needs a native shader")),
    };
    let line = if let Some(l) = &d.line {
        if !l.retained_ordinals.is_empty()
            || l.compound.is_some()
            || l.alignment.is_some()
            || l.head.is_some()
            || l.tail.is_some()
            || matches!(l.join, Some(SourceLineJoin::Miter { .. }))
        {
            return Err(fail("unsupported chart line declaration"));
        }
        if l.dash.as_ref().is_some_and(|d| {
            !matches!(
                d,
                SourceLineDash::Preset {
                    value: Some(NativePresetDash::Solid),
                    ..
                }
            )
        }) {
            return Err(fail("chart dashed lines require native dash geometry"));
        }
        let rgba = match &l.fill {
            None | Some(SourceLineFill::None { .. }) => None,
            Some(SourceLineFill::Solid { color: Some(c), .. }) => {
                Some(color(source, c.source_ordinal)?)
            }
            _ => return Err(fail("unsupported chart line fill")),
        };
        rgba.map(|rgba| {
            let cap = match l.cap {
                Some(NativeLineCap::Round) => mo_raster::StrokeCap::Round,
                Some(NativeLineCap::Square) => mo_raster::StrokeCap::Square,
                _ => mo_raster::StrokeCap::Butt,
            };
            let join = match &l.join {
                Some(SourceLineJoin::Round { .. }) => mo_raster::StrokeJoin::Round {},
                _ => mo_raster::StrokeJoin::Bevel {},
            };
            (
                rgba,
                StrokeStyle {
                    width: Fixed::emu(l.width.unwrap_or(mo_common::Emu::new(9525))),
                    cap,
                    join,
                },
            )
        })
    } else {
        None
    };
    Ok(Paint { fill, line })
}
pub(super) fn point_paint(
    source: &SourceChartPaints,
    series: &SourceChartSeries,
    point: u32,
) -> Result<Paint, SourcePageError> {
    let mut result = paint(source, series.source_ordinal)?;
    if let Some(p) = series.point_overrides.iter().find(|p| p.index == point) {
        let overlay = paint(source, p.source_ordinal)?;
        if let Some(d) = declaration(source, p.source_ordinal) {
            if d.fill.is_some() {
                result.fill = overlay.fill;
            }
            if d.line.is_some() {
                result.line = overlay.line;
            }
        }
    }
    Ok(result)
}
pub(super) fn text(source: &SourceChartPaints, parent: u32) -> Result<TextStyle, SourcePageError> {
    let part = &source.chart.part;
    let body = source
        .chart
        .annotations
        .text_bodies
        .iter()
        .find(|b| b.parent_ordinal == parent)
        .ok_or_else(|| {
            invalid(
                part,
                parent,
                "chart label needs a resolved native text style",
            )
        })?;
    let mut family = None;
    let mut size = None;
    let mut rgba = None;
    let mut bold = false;
    let mut italic = false;
    let mut language = "und".to_owned();
    for (ordinal, node) in &body.styles.nodes {
        if !node.retained_ordinals.is_empty() {
            return Err(invalid(part, *ordinal, "unsupported chart text style"));
        }
        match &node.value {
            V::Character { attributes } if node.element == N::DefRPr => {
                size = attributes.size.map(|s| f64::from(s) * 127.0);
                bold = attributes.bold.unwrap_or(false);
                italic = attributes.italic.unwrap_or(false);
                language = attributes.language.clone().unwrap_or(language);
                let mut other = attributes.as_ref().clone();
                other.size = None;
                other.bold = None;
                other.italic = None;
                other.language = None;
                if other != Default::default() {
                    return Err(invalid(
                        part,
                        *ordinal,
                        "chart text effect requires native mapping",
                    ));
                }
            }
            V::Font { font } => {
                if family.as_ref().is_some_and(|s| s != &font.typeface) {
                    return Err(invalid(
                        part,
                        *ordinal,
                        "mixed script chart fonts require per-run cascade",
                    ));
                }
                family = Some(font.typeface.clone());
            }
            V::Fill { fill } => {
                if let SourceFillDefinition::Solid { color: Some(c) } = &fill.definition {
                    rgba = Some(color(source, c.source_ordinal)?);
                } else {
                    return Err(invalid(
                        part,
                        *ordinal,
                        "chart text fill requires native shader",
                    ));
                }
            }
            V::Body { attributes } => {
                if attributes.rotation.is_some_and(|r| r != 0) || attributes.vertical.is_some() {
                    return Err(invalid(
                        part,
                        *ordinal,
                        "rotated chart text requires layout",
                    ));
                }
            }
            V::Paragraph { attributes } if attributes.as_ref() != &Default::default() => {
                return Err(invalid(
                    part,
                    *ordinal,
                    "chart paragraph formatting requires layout",
                ));
            }
            V::Container {} | V::Paragraph { .. } => (),
            _ => {
                return Err(invalid(
                    part,
                    *ordinal,
                    "unsupported chart text declaration",
                ));
            }
        }
    }
    Ok(TextStyle {
        family: family.ok_or_else(|| invalid(part, parent, "chart font unresolved"))?,
        size: size.ok_or_else(|| invalid(part, parent, "chart font size unresolved"))?,
        bold,
        italic,
        language,
        rgba: rgba.ok_or_else(|| invalid(part, parent, "chart text color unresolved"))?,
    })
}
