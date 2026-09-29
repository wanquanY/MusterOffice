//! Rectangular focus geometry. Exact ratios decide orientation and active edges.
use super::*;
use crate::{interval_extended::floor_ratio, source_number::percentage_ratio};
use mo_raster::GradientField;
use num_bigint::BigInt;

pub(super) fn field(rect: &EffectiveFillRect) -> Result<GradientField, SourcePageError> {
    let values = [&rect.left, &rect.top, &rect.right, &rect.bottom]
        .map(|v| percentage_ratio(&v.value))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            SourcePageError::Invalid("gradient focus percentage lexical limit or range")
        })?;
    rates(&values)
}
fn rates(values: &[(BigInt, BigInt)]) -> Result<GradientField, SourcePageError> {
    for (left, right) in [(0, 2), (1, 3)] {
        let (a, b) = (&values[left], &values[right]);
        // Zero width/height is a valid focus point or line. Compare the exact
        // decimal sum; interval rounding must not reject e.g. 1/3 + 2/3 == 1.
        if &a.0 * &b.1 + &b.0 * &a.1 > &a.1 * &b.1 {
            return Err(SourcePageError::Invalid(
                "inverted rectangular gradient focus",
            ));
        }
    }
    let mut edge_rates = [Fixed::ZERO; 4];
    let mut uncertainty = edge_rates;
    for (i, (n, d)) in values.iter().enumerate() {
        // An outsetting focus edge covers the whole unit tile along that side.
        if n <= &BigInt::from(0) {
            continue;
        }
        let numerator = d << 96usize;
        let reciprocal = I::raw(floor_ratio(&numerator, n), -floor_ratio(&(-numerator), n));
        (edge_rates[i], uncertainty[i]) = reciprocal.q32().map_err(|_| RasterError::Precision)?;
    }
    Ok(GradientField::Rectangular {
        edge_rates,
        uncertainty: Some(uncertainty),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_focus_points_outsets_and_zero_margins_do_not_change_orientation() {
        let values =
            [(1, 3), (0, 1), (2, 3), (-1, 2)].map(|(n, d)| (BigInt::from(n), BigInt::from(d)));
        let GradientField::Rectangular {
            edge_rates,
            uncertainty,
        } = rates(&values).unwrap()
        else {
            panic!()
        };
        assert_eq!(edge_rates.map(Fixed::raw), [3i128 << 32, 0, 3i128 << 31, 0]);
        assert_eq!(uncertainty.unwrap(), [Fixed::ZERO; 4]);
        let mut inverted = values;
        inverted[2].0 += 1;
        assert!(rates(&inverted).is_err());
    }
}
