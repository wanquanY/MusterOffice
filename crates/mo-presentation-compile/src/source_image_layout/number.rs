use super::{ImageLayoutError as E, *};
use crate::{interval_extended::floor_ratio, source_number::*};
use mo_presentation_source::source::drawingml::{NativeCoordinate, NativePercentage};
use num_bigint::BigInt;

fn decimal(s: &str, factor: i64, unit: i32) -> Result<I, E> {
    let (n, d) = decimal_ratio(s, unit).map_err(|e| match e {
        PercentageError::LexicalLimit => E::LexicalLimit,
        PercentageError::Range => E::Range,
    })?;
    let n = (n * factor) << 96usize;
    Ok(I::raw(floor_ratio(&n, &d), -floor_ratio(&(-n), &d)))
}
pub(super) fn percentage(v: &NativePercentage) -> Result<I, E> {
    percentage_interval(v).map_err(|e| match e {
        PercentageError::LexicalLimit => E::LexicalLimit,
        PercentageError::Range => E::Range,
    })
}
pub(super) fn coordinate(v: &NativeCoordinate) -> Result<I, E> {
    let s = v.lexical();
    if s.len() > 256 {
        return Err(E::LexicalLimit);
    }
    for (unit, factor) in [
        ("mm", 36000),
        ("cm", 360000),
        ("in", 914400),
        ("pt", 12700),
        ("pc", 152400),
        ("pi", 152400),
    ] {
        if let Some(s) = s.strip_suffix(unit) {
            return decimal(s, factor, 1);
        }
    }
    decimal(s, 1, 1)
}
pub(super) fn positive(v: &I) -> Result<(), E> {
    if v.hi <= BigInt::from(0) {
        Err(E::Invalid("nonpositive rectangle extent or tile scale"))
    } else if v.lo <= BigInt::from(0) {
        Err(E::Precision)
    } else {
        Ok(())
    }
}
pub(super) fn q32(v: &I) -> Result<(Fixed, Fixed), E> {
    v.q32().map_err(|_| E::Range)
}
pub(super) fn rectangle(v: &[I; 4]) -> Result<(ImageLayoutRectangle, [Fixed; 4]), E> {
    let values: Vec<_> = v.iter().map(q32).collect::<Result<_, _>>()?;
    let rect = ImageLayoutRectangle {
        left: values[0].0,
        top: values[1].0,
        right: values[2].0,
        bottom: values[3].0,
    };
    if rect.left >= rect.right || rect.top >= rect.bottom {
        return Err(E::Precision);
    }
    Ok((rect, std::array::from_fn(|i| values[i].1)))
}
