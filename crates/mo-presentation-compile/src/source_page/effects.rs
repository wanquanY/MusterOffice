//! Compile admitted source effects into domain-independent isolated raster ink.
use super::*;
use mo_presentation_source::source::{
    color::ColorSample,
    effects::{resolve::Resolver, *},
};

pub(super) fn shadow<'a>(
    resolver: &mut Resolver<'a>,
    part: &str,
    id: u32,
    placement: Option<&crate::source_placement::NativePlacement>,
    check: &dyn Fn() -> bool,
) -> Result<Option<mo_raster::OuterShadow>, SourcePageError> {
    let Some(selected) = resolver.select(part, id)? else {
        return Ok(None);
    };
    let at = SourcePageLocation {
        part: selected.part.into(),
        object: Some(id),
    };
    let reject = || {
        mapping(
            &at,
            SourcePageIssue::Effects {
                source_ordinal: selected.properties.source_ordinal,
            },
        )
    };
    if !selected.properties.retained_ordinals.is_empty() {
        return Err(reject());
    }
    let SourceEffectPropertiesDefinition::List { nodes } = &selected.properties.definition else {
        return Err(reject());
    };
    if nodes.is_empty() {
        return Ok(None);
    }
    if nodes.len() != 1 {
        return Err(reject());
    }
    cancel(check)?;
    let node = selected
        .nodes
        .get(&nodes[0])
        .ok_or(SourcePageError::Invalid("effect node binding"))?;
    let SourceEffectDefinition::OuterShadow(s) = &node.definition else {
        return Err(reject());
    };
    if !node.retained_ordinals.is_empty() {
        return Err(reject());
    }
    let percentage =
        |v: &Option<mo_presentation_source::source::drawingml::NativePercentage>| -> bool {
            v.as_ref().is_none_or(|p| {
                let raw = p.lexical();
                let (n, d) = raw
                    .strip_suffix('%')
                    .map_or((raw, 100_000.0), |n| (n, 100.0));
                n.parse::<f64>().ok().is_some_and(|n| n / d == 1.0)
            })
        };
    // Translation and blur are supported. Scaling/skewing the shadow mask is a
    // distinct effect, never approximated by a translated copy.
    if !percentage(&s.scale_x)
        || !percentage(&s.scale_y)
        || s.skew_x.unwrap_or(0) != 0
        || s.skew_y.unwrap_or(0) != 0
    {
        return Err(reject());
    }
    let ColorSample::Resolved { rgba8, .. } = resolver.color(&selected, &s.color)? else {
        return Err(reject());
    };
    let distance = s.distance.map_or(0, |v| v.get()) as f64;
    let angle = f64::from(s.direction.unwrap_or(0)) * std::f64::consts::PI / 10_800_000.0;
    let (mut x, mut y) = (distance * libm::cos(angle), distance * libm::sin(angle));
    if s.rotate_with_shape.unwrap_or(true) {
        let p = placement.ok_or_else(reject)?;
        let a = p.affine.linear.map(|v| v.raw() as f64 / 4294967296.0);
        let nx = libm::hypot(a[0], a[2]);
        let ny = libm::hypot(a[1], a[3]);
        if nx == 0.0 || ny == 0.0 || (a[0] * a[1] + a[2] * a[3]).abs() > nx * ny * 1e-8 {
            return Err(reject());
        }
        (x, y) = (x * a[0] / nx + y * a[1] / ny, x * a[2] / nx + y * a[3] / ny);
    }
    let fixed = |v: f64| -> Result<Fixed, SourcePageError> {
        if !v.is_finite() || v.abs() > 1_099_511_627_776.0 {
            return Err(RasterError::Range.into());
        }
        Ok(Fixed::from_raw(libm::round(v * 4294967296.0) as i128))
    };
    Ok(Some(mo_raster::OuterShadow {
        offset: Point {
            x: fixed(x)?,
            y: fixed(y)?,
        },
        sigma: fixed(s.blur_radius.map_or(0, |v| v.get()) as f64 / 2.0)?,
        rgba: rgba8,
    }))
}
