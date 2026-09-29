use super::*;
use crate::interval::Interval as I;
use mo_presentation_source::source::{
    drawingml::NativeCoordinate, geometry::evaluate::EvaluatedGeometry,
};

fn coordinate(value: &NativeCoordinate) -> Result<I, SourceFrameError> {
    crate::source_number::coordinate_interval(value).map_err(|e| match e {
        crate::source_number::PercentageError::LexicalLimit => {
            SourceFrameError::Limit("frame coordinate lexical bytes")
        }
        crate::source_number::PercentageError::Range => crate::CompileError::Range.into(),
    })
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
    from_values(source, values, body, Fixed::ZERO)
}
pub(super) fn cell(
    geometry: &crate::source_table::TableCellGeometry,
    body: &EffectiveTextBody,
) -> Result<SourceFrameRegion, SourceFrameError> {
    let r = geometry.merged;
    from_values(
        TextRectangleSource::TableCell {
            cell: geometry.address,
            source_ordinal: geometry.source_ordinal,
            region: geometry.region,
        },
        [
            I::fixed(r.min.x),
            I::fixed(r.min.y),
            I::fixed(r.max.x),
            I::fixed(r.max.y),
        ],
        body,
        geometry.conversion_error_bound,
    )
}
fn from_values(
    source: TextRectangleSource,
    values: [I; 4],
    body: &EffectiveTextBody,
    base_error: Fixed,
) -> Result<SourceFrameRegion, SourceFrameError> {
    let mut outer = [Fixed::ZERO; 4];
    let mut outer_error = Fixed::ZERO;
    for (i, v) in values.iter().enumerate() {
        let (at, error) = v.q32()?;
        outer[i] = at;
        outer_error = outer_error.max(error);
    }
    let a = &body.attributes;
    let insets = [&a.left_inset, &a.top_inset, &a.right_inset, &a.bottom_inset];
    let mut out = [Fixed::ZERO; 4];
    let mut error = outer_error;
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
        outer: Rect {
            min: Point {
                x: outer[0],
                y: outer[1],
            },
            max: Point {
                x: outer[2],
                y: outer[3],
            },
        },
        conversion_error_bound: error.checked_add(base_error)?,
    })
}
