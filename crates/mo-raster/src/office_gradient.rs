//! Explicit eligibility, shared by native source selection and raster validation.
use crate::GradientStop;

/// Exact source values decide eligibility, before float32 quantization. A nearby
/// endpoint, asymmetric endpoint color or duplicate middle position must never
/// accidentally opt into Office's special channel curve.
pub fn office_gamma_eligible(stops: &[GradientStop]) -> bool {
    match stops {
        [a, b] => a.position == 0.0 && b.position == 1.0,
        [a, b, c] => {
            a.position == 0.0
                && c.position == 1.0
                && b.position > 0.0
                && b.position < 1.0
                && a.srgb == c.srgb
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradient_tests::{gradient, request};
    use crate::{GradientAlpha, GradientInterpolation, RasterError, compile};

    #[test]
    fn eligibility_uses_exact_endpoint_and_color_values() {
        let mut q = request();
        let g = gradient(&mut q);
        assert!(office_gamma_eligible(&g.stops));
        g.stops.make_mut()[1].position = 1.0f64.next_down();
        assert!(!office_gamma_eligible(&g.stops));
        g.stops.make_mut()[1].position = 1.0;
        let mut stops = g.stops.to_vec();
        stops[1].position = 0.375;
        stops.push(GradientStop {
            position: 1.0,
            srgb: stops[0].srgb,
        });
        assert!(office_gamma_eligible(&stops));
        stops[2].srgb[0] = 1.0f64.next_down();
        assert!(!office_gamma_eligible(&stops));
        stops[2].srgb = stops[0].srgb;
        stops[1].position = 0.0;
        assert!(!office_gamma_eligible(&stops));
        stops[1].position = 1.0;
        assert!(!office_gamma_eligible(&stops));
        stops[1].position = 0.375;
        stops[2].srgb[3] = 0.5;
        assert!(!office_gamma_eligible(&stops));
        stops[2].srgb = stops[0].srgb;
        stops.push(stops[2].clone());
        assert!(!office_gamma_eligible(&stops));
    }

    #[test]
    fn office_curve_is_versioned_without_changing_legacy_ramps() {
        let mut q = request();
        let legacy = compile(&q, &|| false).unwrap();
        gradient(&mut q).interpolation = GradientInterpolation::OfficeGamma1875;
        let office = compile(&q, &|| false).unwrap();
        assert_eq!(office.frame()[1], 10);
        assert_eq!(office.work().gradients, 1);
        let start = 14 + 2 + 7 * q.paths[0].commands.len();
        assert_eq!(&office.frame()[start..start + 5], &[0, 0, 2, 0, 2]);
        gradient(&mut q).alpha = GradientAlpha::Premultiplied;
        assert!(matches!(
            compile(&q, &|| false),
            Err(RasterError::Invalid("Office gradient ramp eligibility"))
        ));
        gradient(&mut q).alpha = GradientAlpha::Straight;
        gradient(&mut q).stops.make_mut()[1].position = 0.99;
        assert!(matches!(
            compile(&q, &|| false),
            Err(RasterError::Invalid("Office gradient ramp eligibility"))
        ));
        gradient(&mut q).stops.make_mut()[1].position = 1.0;
        gradient(&mut q).interpolation = GradientInterpolation::Srgb;
        assert_eq!(compile(&q, &|| false).unwrap().frame(), legacy.frame());
    }

    #[test]
    fn json_cannot_round_a_near_endpoint_into_office_eligibility() {
        let mut q = request();
        let g = gradient(&mut q);
        g.interpolation = GradientInterpolation::OfficeGamma1875;
        g.stops.make_mut()[1].position = 1.0f64.next_down();
        let text = serde_json::to_string(&q).unwrap();
        assert!(text.contains("0.9999999999999999"));
        let parsed: crate::PathRasterRequest = serde_json::from_str(&text).unwrap();
        assert_eq!(q.draws[0].brush, parsed.draws[0].brush);
        assert!(matches!(
            compile(&parsed, &|| false),
            Err(RasterError::Invalid("Office gradient ramp eligibility"))
        ));
        // Check the wire representation directly, independently of this ramp's
        // validator: both neighbors of power-of-two boundaries remain distinct.
        for exponent in [-1022, -100, -10, 0, 10, 100, 1023] {
            let value = 2.0f64.powi(exponent);
            for value in [value.next_down(), value, value.next_up()] {
                let text = serde_json::to_string(&value).unwrap();
                assert_eq!(
                    serde_json::from_str::<f64>(&text).unwrap().to_bits(),
                    value.to_bits()
                );
            }
        }
    }
}
