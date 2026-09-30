//! Shared pie/doughnut angular allocation. Does not guess missing data, negative
//! value behavior, orientation, labels, ring radii or target-application defaults.
use crate::DecimalNumber;
use mo_geometry::Fixed;
use num_bigint::BigUint;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE: &str = "exact-decimal-cumulative-sector-q32-turns-v1-draft";
const TURN: u64 = 1u64 << 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectorWeight {
    pub point_index: u32,
    pub value: DecimalNumber,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NegativeWeights {
    Reject,
    AbsoluteMagnitude,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SectorDirection {
    Clockwise,
    Counterclockwise,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectorRequest {
    /// Q32 turns from twelve o'clock, in [0, 1). No degree/EMU conversion here.
    pub start_turn: Fixed,
    pub direction: SectorDirection,
    pub negative_weights: NegativeWeights,
    /// Explicit drawing order. Stable point indices need not be contiguous.
    pub weights: Vec<SectorWeight>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sector {
    pub point_index: u32,
    /// Unwrapped Q32 turns. Adjacent sectors share exactly the same boundary.
    pub start_turn: Fixed,
    pub end_turn: Fixed,
    pub zero_weight: bool,
    /// A positive weight survives in the plan even if its Q32 endpoints coincide.
    pub positive_below_resolution: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectorLayout {
    pub profile: String,
    pub sectors: Vec<Sector>,
    pub zero_total: bool,
    /// Conservative endpoint error in raw Q32 turn units, relative to start_turn.
    /// For a sweep, combine the two endpoint errors. No geometric error claimed.
    pub endpoint_error_bound: Fixed,
    pub work: SectorWork,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectorWork {
    pub points: u32,
    pub number_bytes: u64,
    /// Sum of exact integer widths after common decimal scaling, including zeros.
    pub scaled_decimal_digits: u64,
    /// Bounds repeated prefix/total arithmetic even if only one weight is wide.
    pub boundary_decimal_digits: u64,
}
#[derive(Debug, Clone, Copy)]
pub struct SectorLimits {
    pub max_points: usize,
    pub max_number_bytes: usize,
    pub max_scaled_decimal_digits: usize,
    pub max_boundary_decimal_digits: usize,
}
impl Default for SectorLimits {
    fn default() -> Self {
        Self {
            max_points: 4096,
            max_number_bytes: 4 * 1024 * 1024,
            max_scaled_decimal_digits: 1_000_000,
            max_boundary_decimal_digits: 4_000_000,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SectorError {
    #[error("invalid sector input: {0}")]
    Invalid(&'static str),
    #[error("duplicate sector point index: {0}")]
    DuplicatePoint(u32),
    #[error("negative sector weight at point: {0}")]
    NegativeWeight(u32),
    #[error("sector computation limit: {0}")]
    Limit(&'static str),
    #[error("sector computation cancelled")]
    Cancelled,
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SectorError> {
    if check() {
        Err(SectorError::Cancelled)
    } else {
        Ok(())
    }
}
fn charge(
    value: &mut u64,
    amount: usize,
    limit: usize,
    name: &'static str,
) -> Result<(), SectorError> {
    *value = value
        .checked_add(amount as u64)
        .filter(|v| *v <= limit as u64)
        .ok_or(SectorError::Limit(name))?;
    Ok(())
}
fn rounded_turn(prefix: &BigUint, total: &BigUint) -> u64 {
    let n = prefix << 32usize;
    let mut q = &n / total;
    if (&n % total) * 2u8 >= *total {
        q += 1u8;
    }
    u64::try_from(q).expect("prefix is between zero and total, Q32 turn fits u64")
}

/// Validate and account the entire request before allocating expanded integers.
/// Failure never returns a prefix layout. All arithmetic is integer/decimal.
pub fn layout(
    request: &SectorRequest,
    limits: SectorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SectorLayout, SectorError> {
    cancel(check)?;
    if !(0..i128::from(TURN)).contains(&request.start_turn.raw()) {
        return Err(SectorError::Invalid("start turn outside [0, 1)"));
    }
    if request.weights.len() > limits.max_points {
        return Err(SectorError::Limit("points"));
    }
    let mut ids = BTreeSet::new();
    let mut work = SectorWork {
        points: request
            .weights
            .len()
            .try_into()
            .map_err(|_| SectorError::Limit("points"))?,
        ..SectorWork::default()
    };
    let mut exponent = None;
    for weight in &request.weights {
        cancel(check)?;
        if !ids.insert(weight.point_index) {
            return Err(SectorError::DuplicatePoint(weight.point_index));
        }
        charge(
            &mut work.number_bytes,
            weight.value.lexical().len(),
            limits.max_number_bytes,
            "number bytes",
        )?;
        if !weight.value.is_zero() {
            if request.negative_weights == NegativeWeights::Reject
                && weight.value.is_sign_negative()
            {
                return Err(SectorError::NegativeWeight(weight.point_index));
            }
            exponent =
                Some(exponent.map_or(weight.value.exponent, |e: i32| e.min(weight.value.exponent)));
        }
    }
    let has_nonzero = exponent.is_some();
    let exponent = exponent.unwrap_or(0);
    let mut widest = 0usize;
    for weight in &request.weights {
        cancel(check)?;
        let width = if weight.value.is_zero() {
            1
        } else {
            weight.value.coefficient_digits + (weight.value.exponent - exponent) as usize
        };
        widest = widest.max(width);
        charge(
            &mut work.scaled_decimal_digits,
            width,
            limits.max_scaled_decimal_digits,
            "scaled decimal digits",
        )?;
    }
    if has_nonzero {
        // Summing N integers grows width by at most decimal_digits(N); shifting
        // a numerator by 32 adds at most ten digits. Account before expansion.
        let width = widest + request.weights.len().to_string().len() + 10;
        let amount = width
            .checked_mul(request.weights.len())
            .ok_or(SectorError::Limit("boundary decimal digits"))?;
        charge(
            &mut work.boundary_decimal_digits,
            amount,
            limits.max_boundary_decimal_digits,
            "boundary decimal digits",
        )?;
    }
    let mut powers = BTreeMap::new();
    let mut scaled = Vec::with_capacity(request.weights.len());
    let mut total = BigUint::from(0u8);
    for weight in &request.weights {
        cancel(check)?;
        let value = if weight.value.is_zero() {
            BigUint::from(0u8)
        } else {
            let power = (weight.value.exponent - exponent) as u32;
            let factor = powers
                .entry(power)
                .or_insert_with(|| BigUint::from(10u8).pow(power));
            &weight.value.coefficient * &*factor
        };
        total += &value;
        scaled.push(value);
    }
    let zero_total = total == BigUint::from(0u8);
    let sign = if request.direction == SectorDirection::Clockwise {
        1i128
    } else {
        -1
    };
    let mut previous = 0u64;
    let mut prefix = BigUint::from(0u8);
    let mut sectors = Vec::with_capacity(scaled.len());
    for (weight, value) in request.weights.iter().zip(scaled) {
        cancel(check)?;
        prefix += value;
        let edge = if zero_total {
            0
        } else {
            rounded_turn(&prefix, &total)
        };
        let at = |t| Fixed::from_raw(request.start_turn.raw() + sign * i128::from(t));
        sectors.push(Sector {
            point_index: weight.point_index,
            start_turn: at(previous),
            end_turn: at(edge),
            zero_weight: weight.value.is_zero(),
            positive_below_resolution: !weight.value.is_zero() && previous == edge,
        });
        previous = edge;
    }
    cancel(check)?;
    Ok(SectorLayout {
        profile: PROFILE.into(),
        sectors,
        zero_total,
        endpoint_error_bound: Fixed::from_raw(if zero_total { 0 } else { 1 }),
        work,
    })
}
