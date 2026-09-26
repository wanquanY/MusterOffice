use super::*;
use crate::{interval::Interval as I, interval_extended::floor_ratio};
use mo_pptx::source::{drawingml::NativeCoordinate, geometry::evaluate::EvaluatedGeometry};
use num_bigint::BigInt;

fn coordinate(value: &NativeCoordinate) -> Result<I, SourceFrameError> {
    let s = value.lexical();
    if s.len() > 256 {
        return Err(SourceFrameError::Limit("frame coordinate lexical bytes"));
    }
    let unit = [
        ("mm", 36000),
        ("cm", 360000),
        ("in", 914400),
        ("pt", 12700),
        ("pc", 152400),
        ("pi", 152400),
    ];
    let (s, factor) = unit
        .iter()
        .find_map(|(u, f)| s.strip_suffix(u).map(|n| (n, *f)))
        .unwrap_or((s, 1));
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    let digits = format!("{whole}{fraction}");
    let n = digits
        .parse::<BigInt>()
        .map_err(|_| crate::CompileError::Range)?
        * factor;
    let d = BigInt::from(10).pow(fraction.len() as u32);
    let n = n << 96usize;
    Ok(I::raw(floor_ratio(&n, &d), -floor_ratio(&(-n), &d)))
}
pub(super) fn region(
    geometry: &EvaluatedGeometry,
    body: &EffectiveTextBody,
) -> Result<SourceFrameRegion, SourceFrameError> {
    let (source, values) = if let Some(r) = &geometry.text_rect {
        (
            TextRectangleSource::Declaration { origin: r.origin },
            [
                I::binary64(r.left)?,
                I::binary64(r.top)?,
                I::binary64(r.right)?,
                I::binary64(r.bottom)?,
            ],
        )
    } else {
        let s = &geometry.extent.value;
        (
            TextRectangleSource::ShapeBounds {
                extent: geometry.extent.clone(),
            },
            [
                I::integer(0),
                I::integer(0),
                I::integer(s.width.get()),
                I::integer(s.height.get()),
            ],
        )
    };
    let a = &body.attributes;
    let insets = [&a.left_inset, &a.top_inset, &a.right_inset, &a.bottom_inset];
    let mut out = [Fixed::ZERO; 4];
    let mut error = Fixed::ZERO;
    for i in 0..4 {
        let inset = coordinate(
            insets[i]
                .as_ref()
                .ok_or_else(|| mapping(SourceFrameIssue::InvalidRegion))?,
        )?;
        let (value, bound) = if i < 2 {
            values[i].add(&inset)
        } else {
            values[i].sub(&inset)
        }
        .q32()?;
        out[i] = value;
        error = error.max(bound);
    }
    if out[2] <= out[0] || out[3] <= out[1] {
        return Err(mapping(SourceFrameIssue::InvalidRegion));
    }
    Ok(SourceFrameRegion {
        source,
        inner: Rect {
            min: Point {
                x: out[0],
                y: out[1],
            },
            max: Point {
                x: out[2],
                y: out[3],
            },
        },
        conversion_error_bound: error,
    })
}
