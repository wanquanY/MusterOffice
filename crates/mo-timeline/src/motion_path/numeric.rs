//! Bounded integer geometry for the paced-path compiler, never host floats.
use crate::MotionPoint;
use num_bigint::{BigInt, BigUint};

pub(super) const UNIT: i128 = 1i128 << 64;
pub(super) const TOLERANCE: i128 = UNIT >> 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Point(pub i128, pub i128);
impl Point {
    pub fn from_source(p: &MotionPoint) -> Self {
        let axis = |c: &crate::MotionCoordinate| {
            let (n, d) = c.parts();
            let d = i128::from(d);
            // Split first: bounded remainders fit i128 even at 18 decimals.
            (n / d) * UNIT + (n % d) * UNIT / d
        };
        Self(axis(&p.x), axis(&p.y))
    }
    pub fn midpoint(self, other: Self) -> Self {
        Self((self.0 + other.0) / 2, (self.1 + other.1) / 2)
    }
    pub fn distance(self, other: Self) -> (i128, i128) {
        let x = BigInt::from(self.0 - other.0);
        let y = BigInt::from(self.1 - other.1);
        let square: BigUint = (&x * &x + &y * &y).try_into().expect("nonnegative");
        let floor = square.sqrt();
        let ceil = &floor * &floor != square;
        // Coordinate bounds imply a distance below 2^96 in Q64.
        let low = i128::try_from(floor).expect("bounded coordinate distance");
        (low, low + i128::from(ceil))
    }
    /// Distance to the *segment*, including degenerate and reversing curves.
    pub fn near_segment(self, a: Self, b: Self) -> bool {
        let dx = BigInt::from(b.0 - a.0);
        let dy = BigInt::from(b.1 - a.1);
        let px = BigInt::from(self.0 - a.0);
        let py = BigInt::from(self.1 - a.1);
        let length2 = &dx * &dx + &dy * &dy;
        let dot = &px * &dx + &py * &dy;
        let tolerance2 = BigInt::from(TOLERANCE).pow(2);
        if dot <= BigInt::from(0) {
            return &px * &px + &py * &py <= tolerance2;
        }
        if dot >= length2 {
            let x = BigInt::from(self.0 - b.0);
            let y = BigInt::from(self.1 - b.1);
            return &x * &x + &y * &y <= tolerance2;
        }
        let cross = &px * &dy - &py * &dx;
        &cross * &cross <= tolerance2 * length2
    }
}

pub(super) fn split(p: [Point; 4]) -> ([Point; 4], [Point; 4]) {
    let a = p[0].midpoint(p[1]);
    let b = p[1].midpoint(p[2]);
    let c = p[2].midpoint(p[3]);
    let d = a.midpoint(b);
    let e = b.midpoint(c);
    let f = d.midpoint(e);
    ([p[0], a, d, f], [f, e, c, p[3]])
}
