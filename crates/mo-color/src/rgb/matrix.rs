#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primaries {
    Bt709,
    Bt2020,
    DisplayP3,
    SmpteC,
    Bt470Bg,
}
/// Row-major matrix, applied to column RGB vectors.
#[derive(Debug, Clone, Copy)]
pub struct Matrix3(pub [[f64; 3]; 3]);
impl Matrix3 {
    pub fn apply(self, v: [f64; 3]) -> [f64; 3] {
        self.0.map(|r| r[0] * v[0] + r[1] * v[1] + r[2] * v[2])
    }
    fn mul(self, b: Self) -> Self {
        let mut out = [[0.; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = (0..3).map(|k| self.0[i][k] * b.0[k][j]).sum();
            }
        }
        Self(out)
    }
    fn inverse(self) -> Self {
        let [[a, b, c], [d, e, f], [g, h, i]] = self.0;
        let cof = [
            [e * i - f * h, c * h - b * i, b * f - c * e],
            [f * g - d * i, a * i - c * g, c * d - a * f],
            [d * h - e * g, b * g - a * h, a * e - b * d],
        ];
        let det = a * cof[0][0] + b * cof[1][0] + c * cof[2][0];
        Self(cof.map(|r| r.map(|v| v / det)))
    }
}
impl Primaries {
    fn xy(self) -> [[f64; 2]; 3] {
        match self {
            Self::Bt709 => [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]],
            Self::Bt2020 => [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]],
            Self::DisplayP3 => [[0.680, 0.320], [0.265, 0.690], [0.150, 0.060]],
            Self::SmpteC => [[0.630, 0.340], [0.310, 0.595], [0.155, 0.070]],
            Self::Bt470Bg => [[0.640, 0.330], [0.290, 0.600], [0.150, 0.060]],
        }
    }
    pub fn to_xyz(self) -> Matrix3 {
        let c = self.xy().map(|[x, y]| [x / y, 1., (1. - x - y) / y]);
        let m = Matrix3([
            [c[0][0], c[1][0], c[2][0]],
            [1.; 3],
            [c[0][2], c[1][2], c[2][2]],
        ]);
        let scale = m
            .inverse()
            .apply([0.3127 / 0.3290, 1., (1. - 0.3127 - 0.3290) / 0.3290]);
        Matrix3(m.0.map(|r| [r[0] * scale[0], r[1] * scale[1], r[2] * scale[2]]))
    }
    pub fn to(self, target: Self) -> Matrix3 {
        if self == target {
            Matrix3([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
        } else {
            target.to_xyz().inverse().mul(self.to_xyz())
        }
    }
    pub fn luminance(self) -> [f64; 3] {
        self.to_xyz().0[1]
    }
}
