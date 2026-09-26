//! Fixed operation order and pinned software libm for native/WASM agreement.
//! Working values stay binary64; no path or wire quantization occurs here.
use super::types::FormulaIssue;
pub(super) const TURN: f64 = 21_600_000.0;
const QUARTER: f64 = TURN / 4.0;
pub(super) fn arity(op: &str) -> Option<usize> {
    Some(match op {
        "val" | "abs" | "sqrt" => 1,
        "at2" | "cos" | "sin" | "tan" | "min" | "max" => 2,
        "*/" | "+-" | "+/" | "?:" | "cat2" | "sat2" | "mod" | "pin" => 3,
        _ => return None,
    })
}
fn radians(angle: f64) -> f64 {
    angle * (core::f64::consts::PI / (TURN / 2.0))
}
fn cos_sin(angle: f64) -> (f64, f64) {
    let a = libm::fmod(angle, TURN);
    if a == 0.0 {
        (1.0, 0.0)
    } else if a == QUARTER || a == -3.0 * QUARTER {
        (0.0, 1.0)
    } else if a.abs() == 2.0 * QUARTER {
        (-1.0, 0.0)
    } else if a == -QUARTER || a == 3.0 * QUARTER {
        (0.0, -1.0)
    } else {
        let r = radians(a);
        (libm::cos(r), libm::sin(r))
    }
}
pub(super) fn compute(op: &str, a: [f64; 3]) -> Result<f64, FormulaIssue> {
    let [x, y, z] = a;
    Ok(match op {
        "val" => x,
        "abs" => x.abs(),
        // Microsoft MS-OE376 2.1.1389 explicitly specifies sqrt(abs(x)).
        "sqrt" => libm::sqrt(x.abs()),
        "*/" | "+/" => {
            if z == 0.0 {
                return Err(FormulaIssue::DivisionByZero);
            }
            if op == "*/" { (x * y) / z } else { (x + y) / z }
        }
        "+-" => (x + y) - z,
        "?:" => {
            if x > 0.0 {
                y
            } else {
                z
            }
        }
        "min" => x.min(y),
        "max" => x.max(y),
        "pin" => {
            if y < x {
                x
            } else if y > z {
                z
            } else {
                y
            }
        }
        "mod" => libm::hypot(libm::hypot(x, y), z),
        "at2" => {
            if x == 0.0 && y == 0.0 {
                return Err(FormulaIssue::UndefinedDirection);
            }
            libm::atan2(y, x) * ((TURN / 2.0) / core::f64::consts::PI)
        }
        "cat2" | "sat2" => {
            if y == 0.0 && z == 0.0 {
                return Err(FormulaIssue::UndefinedDirection);
            }
            // Scaling avoids overflowing y*y + z*z and preserves quadrants.
            let scale = y.abs().max(z.abs());
            let u = y / scale;
            let v = z / scale;
            x * ((if op == "cat2" { u } else { v }) / libm::hypot(u, v))
        }
        "cos" => x * cos_sin(y).0,
        "sin" => x * cos_sin(y).1,
        "tan" => {
            let a = libm::fmod(y, TURN / 2.0);
            if a.abs() == QUARTER {
                return Err(FormulaIssue::TangentPole);
            }
            if a == 0.0 {
                0.0
            } else {
                x * libm::tan(radians(a))
            }
        }
        _ => return Err(FormulaIssue::UnknownOperation),
    })
}
/// Decimal literals only, not NaN/Inf or host-language expressions.
pub(super) fn decimal(s: &str) -> Option<f64> {
    let unsigned = s.strip_prefix(['+', '-']).unwrap_or(s);
    let mut digits = 0;
    let mut dots = 0;
    for c in unsigned.bytes() {
        if c.is_ascii_digit() {
            digits += 1;
        } else if c == b'.' {
            dots += 1;
        } else {
            return None;
        }
    }
    if digits == 0 || dots > 1 {
        return None;
    }
    s.parse().ok()
}
