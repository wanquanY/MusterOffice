//! Shared native stroke-property inheritance. Paint is evaluated by the fill engine.
use super::{super::*, *};
use crate::source::drawingml::*;
fn value<T>(value: T, source: &LineOrigin) -> LineValue<T> {
    LineValue {
        value,
        declared_by: source.clone(),
    }
}
fn set<T: Clone>(slot: &mut Option<LineValue<T>>, input: &Option<T>, origin: &LineOrigin) {
    if slot.is_none() {
        *slot = input.as_ref().map(|v| value(v.clone(), origin));
    }
}
#[derive(Default)]
struct End {
    origin: Option<LineOrigin>,
    kind: Option<LineValue<NativeLineEnd>>,
    width: Option<LineValue<NativeLineEndSize>>,
    length: Option<LineValue<NativeLineEndSize>>,
}
impl End {
    fn merge(&mut self, end: &SourceLineEnd, origin: &LineOrigin) {
        let origin = origin.at(end.source_ordinal);
        self.origin.get_or_insert_with(|| origin.clone());
        set(&mut self.kind, &end.kind, &origin);
        set(&mut self.width, &end.width, &origin);
        set(&mut self.length, &end.length, &origin);
    }
    fn complete(&self) -> bool {
        self.kind.is_some() && self.width.is_some() && self.length.is_some()
    }
    fn finish(self) -> EffectiveLineEnd {
        let default = LineOrigin::ProfileDefault {};
        EffectiveLineEnd {
            declared_by: self.origin.unwrap_or(default.clone()),
            kind: self
                .kind
                .unwrap_or_else(|| value(NativeLineEnd::None, &default)),
            width: self
                .width
                .unwrap_or_else(|| value(NativeLineEndSize::Medium, &default)),
            length: self
                .length
                .unwrap_or_else(|| value(NativeLineEndSize::Medium, &default)),
        }
    }
}
enum Dash {
    Preset(LineOrigin, Option<LineValue<NativePresetDash>>),
    Custom(LineOrigin, Vec<SourceDashStop>),
}
enum Join {
    Round(LineOrigin),
    Bevel(LineOrigin),
    Miter(LineOrigin, Option<LineValue<NativePercentage>>),
}
#[derive(Default)]
pub(in crate::source) struct GeometryPartial {
    width: Option<LineValue<Emu>>,
    cap: Option<LineValue<NativeLineCap>>,
    compound: Option<LineValue<NativeCompoundLine>>,
    alignment: Option<LineValue<NativePenAlignment>>,
    dash: Option<Dash>,
    join: Option<Join>,
    head: End,
    tail: End,
}
impl GeometryPartial {
    pub fn complete(&self) -> bool {
        self.width.is_some()
            && self.cap.is_some()
            && self.compound.is_some()
            && self.alignment.is_some()
            && matches!(
                self.dash,
                Some(Dash::Preset(_, Some(_)) | Dash::Custom(_, _))
            )
            && matches!(
                self.join,
                Some(Join::Round(_) | Join::Bevel(_) | Join::Miter(_, Some(_)))
            )
            && self.head.complete()
            && self.tail.complete()
    }
    pub fn merge(&mut self, input: &SourceLine, origin: &LineOrigin) -> Result<(), LineUnresolved> {
        if !input.retained_ordinals.is_empty() && !self.complete() {
            return Err(LineUnresolved::RetainedContent {
                origin: origin.at(input.retained_ordinals[0]),
            });
        }
        set(&mut self.width, &input.width, origin);
        set(&mut self.cap, &input.cap, origin);
        set(&mut self.compound, &input.compound, origin);
        set(&mut self.alignment, &input.alignment, origin);
        if let Some(dash) = &input.dash {
            match dash {
                SourceLineDash::Preset {
                    source_ordinal,
                    value: native,
                } => {
                    let origin = origin.at(*source_ordinal);
                    if self.dash.is_none() {
                        self.dash = Some(Dash::Preset(origin.clone(), None));
                    }
                    if let Some(Dash::Preset(_, slot)) = &mut self.dash {
                        set(slot, native, &origin);
                    }
                }
                SourceLineDash::Custom {
                    source_ordinal,
                    stops,
                } => {
                    if self.dash.is_none() {
                        self.dash = Some(Dash::Custom(origin.at(*source_ordinal), stops.clone()));
                    }
                }
            }
        }
        if let Some(join) = &input.join {
            match join {
                SourceLineJoin::Round { source_ordinal } => {
                    if self.join.is_none() {
                        self.join = Some(Join::Round(origin.at(*source_ordinal)));
                    }
                }
                SourceLineJoin::Bevel { source_ordinal } => {
                    if self.join.is_none() {
                        self.join = Some(Join::Bevel(origin.at(*source_ordinal)));
                    }
                }
                SourceLineJoin::Miter {
                    source_ordinal,
                    limit,
                } => {
                    let origin = origin.at(*source_ordinal);
                    if self.join.is_none() {
                        self.join = Some(Join::Miter(origin.clone(), None));
                    }
                    if let Some(Join::Miter(_, slot)) = &mut self.join {
                        set(slot, limit, &origin);
                    }
                }
            }
        }
        if let Some(end) = &input.head {
            self.head.merge(end, origin);
        }
        if let Some(end) = &input.tail {
            self.tail.merge(end, origin);
        }
        Ok(())
    }
    pub fn finish(self) -> EffectiveLineGeometry {
        let default = LineOrigin::ProfileDefault {};
        let dash = match self.dash.unwrap_or(Dash::Preset(default.clone(), None)) {
            Dash::Preset(declared_by, preset) => EffectiveLineDash::Preset {
                declared_by,
                value: preset.unwrap_or_else(|| value(NativePresetDash::Solid, &default)),
            },
            Dash::Custom(declared_by, stops) => EffectiveLineDash::Custom { declared_by, stops },
        };
        let join = match self.join.unwrap_or(Join::Round(default.clone())) {
            Join::Round(declared_by) => EffectiveLineJoin::Round { declared_by },
            Join::Bevel(declared_by) => EffectiveLineJoin::Bevel { declared_by },
            Join::Miter(declared_by, limit) => EffectiveLineJoin::Miter {
                declared_by,
                limit: limit.unwrap_or_else(|| {
                    value(
                        "800000"
                            .to_owned()
                            .try_into()
                            .expect("documented positive percentage"),
                        &default,
                    )
                }),
            },
        };
        EffectiveLineGeometry {
            width: self
                .width
                .unwrap_or_else(|| value(Emu::new(9525), &default)),
            cap: self
                .cap
                .unwrap_or_else(|| value(NativeLineCap::Flat, &default)),
            compound: self
                .compound
                .unwrap_or_else(|| value(NativeCompoundLine::Single, &default)),
            alignment: self
                .alignment
                .unwrap_or_else(|| value(NativePenAlignment::Center, &default)),
            dash,
            join,
            head: self.head.finish(),
            tail: self.tail.finish(),
        }
    }
}
