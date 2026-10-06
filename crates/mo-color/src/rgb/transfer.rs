use super::{REFERENCE_WHITE_NITS, RgbError, finite};
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transfer {
    Linear,
    Srgb,
    /// Ideal-black BT.1886 reference-display EOTF for SDR television.
    Bt1886,
    Gamma22,
    Gamma28,
    Pq,
    /// BT.2100 reference-display EOTF. Peak and system gamma are explicit;
    /// inverse OETF alone is not an HLG display-light conversion.
    Hlg {
        peak_nits: f64,
        system_gamma: f64,
    },
}
impl Transfer {
    pub fn is_hdr(self) -> bool {
        matches!(self, Self::Pq | Self::Hlg { .. })
    }
    pub fn validate(self) -> Result<(), RgbError> {
        if let Self::Hlg {
            peak_nits,
            system_gamma,
        } = self
            && (!(100.0..=10000.0).contains(&peak_nits) || !(0.8..=1.5).contains(&system_gamma))
        {
            return Err(RgbError::Parameter);
        }
        Ok(())
    }
    pub(super) fn decode(
        self,
        mut rgb: [f64; 3],
        weights: [f64; 3],
    ) -> Result<([f64; 3], bool), RgbError> {
        self.validate()?;
        finite(rgb)?;
        let mut clipped = false;
        if self.is_hdr() {
            for v in &mut rgb {
                if !(0.0..=1.0).contains(v) {
                    clipped = true;
                    *v = v.clamp(0., 1.);
                }
            }
        }
        if matches!(self, Self::Bt1886) {
            for v in &mut rgb {
                if *v < 0. {
                    clipped = true;
                    *v = 0.;
                }
            }
        }
        let linear = match self {
            Self::Linear => rgb,
            Self::Srgb => rgb.map(crate::srgb_to_linear),
            Self::Gamma22 => rgb.map(|v| signed_pow(v, 2.2)),
            Self::Gamma28 => rgb.map(|v| signed_pow(v, 2.8)),
            Self::Bt1886 => rgb.map(|v| libm::pow(v, 2.4)),
            Self::Pq => rgb.map(pq_to_nits).map(|v| v / REFERENCE_WHITE_NITS),
            Self::Hlg {
                peak_nits,
                system_gamma,
            } => {
                let scene = rgb.map(hlg_inverse_oetf);
                let y =
                    (scene[0] * weights[0] + scene[1] * weights[1] + scene[2] * weights[2]).max(0.);
                if y == 0. {
                    [0.; 3]
                } else {
                    let gain = libm::pow(y, system_gamma - 1.) * peak_nits / REFERENCE_WHITE_NITS;
                    scene.map(|v| v * gain)
                }
            }
        };
        finite(linear)?;
        Ok((linear, clipped))
    }
}
fn signed_pow(v: f64, p: f64) -> f64 {
    libm::pow(v.abs(), p).copysign(v)
}
fn pq_to_nits(v: f64) -> f64 {
    let m1 = 2610. / 16384.;
    let m2 = 2523. / 32.;
    let c1 = 3424. / 4096.;
    let c2 = 2413. / 128.;
    let c3 = 2392. / 128.;
    let p = libm::pow(v, 1. / m2);
    10000. * libm::pow((p - c1).max(0.) / (c2 - c3 * p), 1. / m1)
}
fn hlg_inverse_oetf(v: f64) -> f64 {
    const A: f64 = 0.17883277;
    let b = 1. - 4. * A;
    let c = 0.5 - A * libm::log(4. * A);
    if v <= 0.5 {
        v * v / 3.
    } else {
        (libm::exp((v - c) / A) + b) / 12.
    }
}
