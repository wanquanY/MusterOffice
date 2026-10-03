//! Native chart resources. Computed paths retain real chart/source identities;
//! neither the inspected index nor the original OPC package is rewritten.
mod errors;
mod layout;
mod style;
pub(crate) mod text;
use crate::{path_scene::SceneBuilder, source_page::*, source_resource_page::TextPageContext};
use mo_common::Digest;
use mo_geometry::{Fixed, PathCommand as C, Point};
use mo_opc::PackageRead;
use mo_presentation_source::source::{
    SourceIndex, SourceObjectRef,
    charts::{self, *},
};
use mo_raster::{Brush, RasterError, StrokeStyle};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(crate) type Charts = BTreeMap<(String, u32), ChartPage>;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartPageInfo {
    pub object: SourceObjectRef,
    pub chart_part: String,
    pub chart_sha256: Digest,
    pub data_authority: ChartDataAuthority,
    pub native_kind: String,
    pub labels: u32,
    pub paths: u32,
    pub text_work: crate::source_frame::FrameWork,
    pub path_bytes: u64,
}
pub(crate) struct ChartPage {
    pub info: ChartPageInfo,
    pub size: mo_presentation_model::Size,
    pub draws: Vec<Draw>,
    labels: Vec<Label>,
}
pub(super) struct Draw {
    pub ordinal: u32,
    pub commands: Vec<C>,
    pub origin: Point,
    pub rgba: [u8; 4],
    pub stroke: Option<StrokeStyle>,
    pub error: Fixed,
}
struct Label {
    pub ordinal: u32,
    pub text: String,
    pub style: style::TextStyle,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub align: f64,
}
struct Layout {
    pub draws: Vec<Draw>,
    pub labels: Vec<Label>,
}
pub(super) fn invalid(part: &str, ordinal: u32, reason: impl Into<String>) -> SourcePageError {
    SourcePageError::Chart {
        part: part.into(),
        source_ordinal: ordinal,
        reason: reason.into(),
    }
}
pub(super) fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourcePageError> {
    if check() {
        Err(RasterError::Cancelled.into())
    } else {
        Ok(())
    }
}
/// Values are bounded to the native slide range. The layout stage accounts a
/// conservative 1/1024 EMU error for binary64 arithmetic before Q32 conversion.
pub(super) fn fixed(v: f64) -> Result<Fixed, SourcePageError> {
    if !v.is_finite() || v.abs() > 1_000_000_000.0 {
        return Err(RasterError::Range.into());
    }
    Ok(Fixed::from_raw((v * 4294967296.0).round() as i128))
}
pub(super) fn point(x: f64, y: f64) -> Result<Point, SourcePageError> {
    Ok(Point {
        x: fixed(x)?,
        y: fixed(y)?,
    })
}
pub(super) const ERROR: Fixed = Fixed::from_raw(1 << 22);

pub(crate) fn prepare(
    package: &dyn PackageRead,
    index: &SourceIndex,
    q: &SourcePageRequest,
    properties: Option<&crate::source_placement::SourceProperties>,
    context: &mut Option<TextPageContext<'_, '_, '_>>,
    check: &dyn Fn() -> bool,
) -> Result<Arc<Charts>, SourcePageError> {
    // SourcePlan declarations may carry the semantic edit digest. Native chart
    // parts remain pinned to the verified underlying package; all placement and
    // visibility below still use the current declarations.
    let raw_index = if index.source_sha256 != *package.sha256() {
        let mut raw = index.clone();
        raw.source_sha256 = package.sha256().clone();
        Some(raw)
    } else {
        None
    };
    let chart_index = raw_index.as_ref().unwrap_or(index);
    let slide = index
        .surfaces
        .get(&q.slide)
        .ok_or(SourcePageError::Invalid("chart slide"))?;
    let mut surfaces = vec![q.slide.clone()];
    if let Some(layout) = &slide.links.layout {
        surfaces.push(layout.clone());
        if let Some(master) = &index.surfaces[layout].links.master {
            surfaces.push(master.clone());
        }
    }
    let mut bindings = BTreeMap::new();
    for surface in surfaces {
        cancel(check)?;
        if !index.surfaces[&surface].objects.iter().any(|o| {
            o.kind == mo_presentation_source::source::SourceObjectKind::GraphicFrame
                && o.table.is_none()
        }) {
            continue;
        }
        let source = charts::query(
            package,
            chart_index,
            &SourceChartQuery {
                expected_source_sha256: package.sha256().clone(),
                surface,
            },
            Default::default(),
            Default::default(),
            check,
        )?;
        for binding in source.bindings {
            if bindings.len() >= 64 {
                return Err(RasterError::Limit("page charts").into());
            }
            bindings.insert(
                (binding.object.part.clone(), binding.object.native_id),
                binding,
            );
        }
    }
    if bindings.is_empty() {
        return Ok(Arc::default());
    }
    let keys: BTreeSet<_> = bindings.keys().cloned().collect();
    let layers = crate::source_page::chart_layers(
        index,
        q,
        context.is_some(),
        true,
        properties,
        &keys,
        check,
    )?;
    let mut plans = Vec::new();
    let mut label_count = 0usize;
    let mut draw_count = 0usize;
    let mut path_bytes = 0u64;
    for layer in layers {
        for id in layer.objects {
            let key = (layer.part.clone(), id);
            let Some(binding) = bindings.get(&key) else {
                continue;
            };
            cancel(check)?;
            let paints = charts::paints::query(
                package,
                chart_index,
                &charts::paints::SourceChartPaintQuery {
                    expected_source_sha256: package.sha256().clone(),
                    object: binding.object.clone(),
                    profile:
                        mo_presentation_source::source::color::ColorProfile::Ecma3762016DraftV1,
                    context: q.color_context.clone(),
                },
                Default::default(),
                Default::default(),
                check,
            )?;
            let native = index.surfaces[&layer.part]
                .objects
                .iter()
                .find(|o| o.native_id == id)
                .expect("bound chart");
            let size = native
                .transform
                .as_ref()
                .and_then(|t| t.size)
                .ok_or_else(|| {
                    invalid(
                        &paints.chart.part,
                        binding.source_ordinal,
                        "chart extent missing",
                    )
                })?;
            let area = charts::page::page_structure(package, &paints.chart.part, check)?;
            let layout = layout::prepare(&paints, area, size, check)?;
            label_count = label_count
                .checked_add(layout.labels.len())
                .ok_or(RasterError::Range)?;
            draw_count = draw_count
                .checked_add(layout.draws.len())
                .ok_or(RasterError::Range)?;
            if label_count > 4096 || draw_count > 16384 {
                return Err(RasterError::Limit("chart page layout").into());
            }
            for draw in &layout.draws {
                charge_paths(&mut path_bytes, draw.commands.len())?;
            }
            plans.push((key, paints, size, layout));
        }
    }
    // All data/style/geometry admission precedes component calls. Validate every
    // paragraph's manifest binding before shaping the first one.
    if label_count > 0 {
        let context = context
            .as_ref()
            .ok_or(SourcePageError::Invalid("chart font context required"))?;
        for (_, _, _, layout) in &plans {
            text::validate(&layout.labels, context, check)?;
        }
    }
    let mut result = Charts::new();
    for (key, paints, size, layout) in plans {
        let labels = layout.labels.len() as u32;
        let info = ChartPageInfo {
            object: paints.object,
            chart_part: paints.chart.part,
            chart_sha256: paints.chart.sha256,
            data_authority: paints.chart.data_authority,
            native_kind: paints.chart.plots[0].native_kind.clone(),
            labels,
            paths: layout.draws.len() as u32,
            text_work: Default::default(),
            path_bytes: layout
                .draws
                .iter()
                .map(|d| 160 + d.commands.len() as u64 * 112)
                .sum(),
        };
        result.insert(
            key,
            ChartPage {
                info,
                size,
                draws: layout.draws,
                labels: layout.labels,
            },
        );
    }
    Ok(Arc::new(result))
}

pub(super) fn charge_paths(bytes: &mut u64, commands: usize) -> Result<(), SourcePageError> {
    let cost = (commands as u64)
        .checked_mul(112)
        .and_then(|n| n.checked_add(160))
        .ok_or(RasterError::Range)?;
    *bytes = bytes
        .checked_add(cost)
        .filter(|n| *n <= 64 * 1024 * 1024)
        .ok_or(RasterError::Limit("chart retained paths"))?;
    Ok(())
}
impl ChartPage {
    pub(crate) fn emit(
        &self,
        binding: u32,
        object: &SourcePagePaintBinding,
        builder: &mut SceneBuilder<SourcePagePaintSource>,
        viewport: &mo_raster::RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(Fixed, Fixed), SourcePageError> {
        if object
            .placement
            .as_ref()
            .is_none_or(|p| p.source_size != self.size)
        {
            return Err(SourcePageError::Invalid(
                "retained chart dimensions changed",
            ));
        }
        let mut position = Fixed::ZERO;
        let mut geometry = Fixed::ZERO;
        for draw in &self.draws {
            cancel(check)?;
            let (affine, p, g) = crate::source_text_page::placement::transform(
                &draw.commands,
                draw.origin,
                draw.error,
                object,
                viewport,
                check,
            )?;
            position = position.max(p);
            geometry = geometry.max(g);
            builder.add(
                &draw.commands,
                affine,
                Brush::Solid { rgba: draw.rgba },
                draw.stroke,
                |instance| SourcePagePaintSource {
                    instance,
                    binding,
                    path: None,
                    paint: if draw.stroke.is_some() {
                        crate::PagePaintKind::Stroke
                    } else {
                        crate::PagePaintKind::Fill
                    },
                    fill_target: None,
                    chart: Some(ChartPaintSource {
                        part: self.info.chart_part.clone(),
                        source_ordinal: draw.ordinal,
                    }),
                },
            )?;
        }
        Ok((position, geometry))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartPaintSource {
    pub part: String,
    pub source_ordinal: u32,
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod retained_tests;
