use super::{Failure, builtin, math, types::*};
use crate::{PptxError, source::geometry::*};
use std::collections::BTreeMap;

pub(super) struct Budget<'a> {
    pub limits: GeometryLimits,
    pub check: &'a dyn Fn() -> bool,
    pub steps: usize,
    pub bytes: usize,
    pub values: usize,
}
impl Budget<'_> {
    pub fn step(&mut self) -> Result<(), PptxError> {
        crate::cancelled(self.check)?;
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(PptxError::Limit("geometry evaluation steps"))?;
        if self.steps > self.limits.max_steps {
            return Err(PptxError::Limit("geometry evaluation steps"));
        }
        Ok(())
    }
    pub fn lexical(&mut self, text: &str) -> Result<(), PptxError> {
        self.bytes = self
            .bytes
            .checked_add(text.len())
            .ok_or(PptxError::Limit("geometry evaluation bytes"))?;
        if self.bytes > self.limits.max_lexical_bytes {
            return Err(PptxError::Limit("geometry evaluation bytes"));
        }
        Ok(())
    }
    pub fn value(&mut self) -> Result<(), PptxError> {
        self.step()?;
        self.values = self
            .values
            .checked_add(1)
            .ok_or(PptxError::Limit("geometry evaluation values"))?;
        if self.values > self.limits.max_values {
            return Err(PptxError::Limit("geometry evaluation values"));
        }
        Ok(())
    }
}
pub(super) struct Session<'a, 'b, 'c> {
    pub budget: &'b mut Budget<'c>,
    pub width: f64,
    pub height: f64,
    pub preset: Option<NativeShapeType>,
    pub document_guides: bool,
    values: BTreeMap<&'a str, (f64, GeometryOrigin)>,
    adjustments: BTreeMap<&'a str, GeometryOrigin>,
}
pub(super) fn finite(value: f64, origin: GeometryOrigin) -> Result<f64, Failure> {
    if !value.is_finite() {
        return Err(GeometryUnresolved::NumericRange { origin }.into());
    }
    // Do not expose signed zero differences in public geometry values.
    Ok(if value == 0.0 { 0.0 } else { value })
}
pub(super) fn space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}
impl<'a, 'b, 'c> Session<'a, 'b, 'c> {
    pub fn new(width: f64, height: f64, budget: &'b mut Budget<'c>) -> Self {
        Self {
            budget,
            width,
            height,
            preset: None,
            document_guides: false,
            values: BTreeMap::new(),
            adjustments: BTreeMap::new(),
        }
    }
    // Borrow the lookup token; coordinate evaluation needs no allocated
    // dependency record. Formula results materialize only their own edges.
    fn reference(&self, token: &str) -> Option<(f64, Option<GeometryOrigin>)> {
        if let Some((value, source_ordinal)) = self.values.get(token) {
            Some((*value, Some(*source_ordinal)))
        } else {
            self.builtin(token).map(|value| (value, None))
        }
    }
    pub fn origin(&self, ordinal: u32) -> GeometryOrigin {
        match self.preset.filter(|_| !self.document_guides) {
            Some(preset) => GeometryOrigin::Preset {
                preset,
                definition_ordinal: ordinal,
            },
            None => GeometryOrigin::document(ordinal),
        }
    }
    fn builtin(&self, name: &str) -> Option<f64> {
        builtin::value(name, self.width, self.height).or_else(|| {
            if self.preset.is_none() || self.document_guides {
                return None;
            }
            // The published preset appendix references these additional ratios.
            // They are catalog-local; arbitrary custom geometry stays strict.
            match name {
                "wd12" => Some(self.width / 12.0),
                "wd32" => Some(self.width / 32.0),
                "hd10" => Some(self.height / 10.0),
                "cd3" => Some(7_200_000.0),
                _ => None,
            }
        })
    }
    fn unknown(&self, token: &str, source_ordinal: u32) -> Failure {
        GeometryUnresolved::UnknownReference {
            origin: self.origin(source_ordinal),
            token: token.into(),
        }
        .into()
    }
    pub fn guides(
        &mut self,
        list: Option<&'a SourceGeometryList<SourceGuide>>,
        adjustments: bool,
    ) -> Result<Vec<GuideValue>, Failure> {
        let mut out = Vec::new();
        if let Some(list) = list {
            for guide in &list.entries {
                self.budget.value()?;
                self.budget.lexical(&guide.name)?;
                self.budget.lexical(&guide.formula)?;
                let name = guide.name.trim_matches(space);
                let ordinal = guide.source_ordinal;
                if name.is_empty() || name.chars().any(space) || math::decimal(name).is_some() {
                    return Err(GeometryUnresolved::InvalidGuideName {
                        origin: self.origin(ordinal),
                    }
                    .into());
                }
                if self.builtin(name).is_some() {
                    return Err(GeometryUnresolved::ReservedGuide {
                        origin: self.origin(ordinal),
                    }
                    .into());
                }
                let mut words = guide.formula.split(space).filter(|s| !s.is_empty());
                let op = words.next().unwrap_or("");
                let origin = self.origin(ordinal);
                let failure = |issue| GeometryUnresolved::Formula { origin, issue };
                let count =
                    math::arity(op).ok_or_else(|| failure(FormulaIssue::UnknownOperation))?;
                let mut args = [0.0; 3];
                let mut dependencies = Vec::new();
                // Each native formula has at most three operands. No temporary
                // token vector or recursive forward-reference evaluation.
                for arg in args.iter_mut().take(count) {
                    self.budget.step()?;
                    let token = words.next().ok_or_else(|| failure(FormulaIssue::Arity))?;
                    if let Some((v, dep)) = self.reference(token) {
                        *arg = v;
                        dependencies.push(match dep {
                            Some(origin) => GuideDependency::Guide { origin },
                            None => GuideDependency::Builtin { name: token.into() },
                        });
                    } else if let Some(v) = math::decimal(token) {
                        *arg = finite(v, self.origin(ordinal))?;
                    } else {
                        return Err(self.unknown(token, ordinal));
                    }
                }
                if words.next().is_some() {
                    return Err(failure(FormulaIssue::Arity).into());
                }
                let value = finite(
                    math::compute(op, args).map_err(failure)?,
                    self.origin(ordinal),
                )?;
                self.values.insert(name, (value, self.origin(ordinal)));
                if adjustments {
                    self.adjustments.insert(name, self.origin(ordinal));
                }
                out.push(GuideValue {
                    origin: self.origin(ordinal),
                    name: name.into(),
                    value,
                    dependencies,
                });
            }
        }
        Ok(out)
    }
    pub fn coordinate(&mut self, raw: &str, ordinal: u32) -> Result<f64, Failure> {
        self.budget.value()?;
        self.budget.lexical(raw)?;
        let token = raw.trim_matches(space);
        if let Some((v, _)) = self.reference(token) {
            return Ok(v);
        }
        if let Ok(n) = token.parse::<i64>() {
            if !(-27_273_042_329_600..=27_273_042_316_900).contains(&n) {
                return Err(GeometryUnresolved::InvalidCoordinate {
                    origin: self.origin(ordinal),
                }
                .into());
            }
            return Ok(n as f64);
        }
        for (suffix, factor) in [
            ("mm", 36_000.0),
            ("cm", 360_000.0),
            ("in", 914_400.0),
            ("pt", 12_700.0),
            ("pc", 152_400.0),
            ("pi", 152_400.0),
        ] {
            if let Some(number) = token.strip_suffix(suffix) {
                let value = math::decimal(number).ok_or(GeometryUnresolved::InvalidCoordinate {
                    origin: self.origin(ordinal),
                })?;
                return finite(value * factor, self.origin(ordinal));
            }
        }
        Err(self.unknown(token, ordinal))
    }
    pub fn angle(&mut self, raw: &str, ordinal: u32) -> Result<f64, Failure> {
        self.budget.value()?;
        self.budget.lexical(raw)?;
        let token = raw.trim_matches(space);
        if let Some((v, _)) = self.reference(token) {
            return Ok(v);
        }
        if let Ok(n) = token.parse::<i32>() {
            return Ok(f64::from(n));
        }
        if math::decimal(token).is_some() {
            return Err(GeometryUnresolved::InvalidAngle {
                origin: self.origin(ordinal),
            }
            .into());
        }
        Err(self.unknown(token, ordinal))
    }
    pub fn optional(
        &mut self,
        raw: Option<&str>,
        ordinal: u32,
        angle: bool,
    ) -> Result<Option<f64>, Failure> {
        raw.map(|raw| {
            if angle {
                self.angle(raw, ordinal)
            } else {
                self.coordinate(raw, ordinal)
            }
        })
        .transpose()
    }
    pub fn handle_reference(
        &mut self,
        raw: Option<&str>,
        ordinal: u32,
    ) -> Result<Option<GeometryOrigin>, Failure> {
        raw.map(|raw| {
            self.budget.step()?;
            self.budget.lexical(raw)?;
            self.adjustments
                .get(raw.trim_matches(space))
                .copied()
                .ok_or_else(|| {
                    GeometryUnresolved::InvalidHandleReference {
                        origin: self.origin(ordinal),
                    }
                    .into()
                })
        })
        .transpose()
    }
    pub fn point(&mut self, p: &SourceGeometryPoint) -> Result<EvaluatedPoint, Failure> {
        Ok(EvaluatedPoint {
            origin: self.origin(p.source_ordinal),
            x: self.coordinate(&p.x, p.source_ordinal)?,
            y: self.coordinate(&p.y, p.source_ordinal)?,
        })
    }
}
