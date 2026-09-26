use super::*;

#[derive(Default)]
pub(super) struct Gradient {
    stops: Option<FillValue<Vec<EffectiveGradientStop>>>,
    shade: Option<Shade>,
    tile_rect: Option<EffectiveFillRect>,
    flip: Option<FillValue<NativeTileFlip>>,
    rotate: Option<FillValue<bool>>,
}
enum Shade {
    Linear {
        origin: FillOrigin,
        angle: Option<FillValue<u32>>,
        scaled: Option<FillValue<bool>>,
    },
    Path {
        origin: FillOrigin,
        path: Option<FillValue<NativePathShade>>,
        rect: Option<Box<InheritedRect>>,
    },
}
impl Gradient {
    pub fn complete(&self) -> bool {
        self.stops.is_some()
            && self.tile_rect.is_some()
            && self.flip.is_some()
            && self.rotate.is_some()
            && match &self.shade {
                Some(Shade::Linear { angle, scaled, .. }) => angle.is_some() && scaled.is_some(),
                Some(Shade::Path { path, rect, .. }) => {
                    path.is_some() && rect.as_ref().is_some_and(|r| r.complete())
                }
                None => false,
            }
    }
    pub fn merge(
        &mut self,
        v: &SourceGradientFill,
        origin: &FillOrigin,
        context: Option<&FillOwner>,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        set(&mut self.flip, &v.flip, origin, budget)?;
        set(&mut self.rotate, &v.rotate_with_shape, origin, budget)?;
        if self.stops.is_none()
            && let Some(stops) = &v.stops
        {
            budget.values(stops.entries.len())?;
            let mut entries = Vec::with_capacity(stops.entries.len());
            for stop in &stops.entries {
                let at = budget.at(origin, stop.source_ordinal)?;
                entries.push(EffectiveGradientStop {
                    position: budget.bind(&stop.position, &at)?,
                    color: color(&stop.color, origin, context, budget)?,
                });
            }
            self.stops = Some(FillValue {
                value: entries,
                declared_by: budget.at(origin, stops.source_ordinal)?,
            });
        }
        if self.tile_rect.is_none()
            && let Some(v) = &v.tile_rect
        {
            self.tile_rect = Some(rect(v, origin, budget)?);
        }
        if let Some(v) = &v.shade {
            if self.shade.is_none() {
                self.shade = Some(match v {
                    SourceGradientShade::Linear { source_ordinal, .. } => Shade::Linear {
                        origin: budget.at(origin, *source_ordinal)?,
                        angle: None,
                        scaled: None,
                    },
                    SourceGradientShade::Path { source_ordinal, .. } => Shade::Path {
                        origin: budget.at(origin, *source_ordinal)?,
                        path: None,
                        rect: None,
                    },
                });
            }
            match (self.shade.as_mut().expect("shade selected"), v) {
                (
                    Shade::Linear { angle, scaled, .. },
                    SourceGradientShade::Linear {
                        source_ordinal,
                        angle: a,
                        scaled: s,
                    },
                ) => {
                    let at = budget.at(origin, *source_ordinal)?;
                    set(angle, a, &at, budget)?;
                    set(scaled, s, &at, budget)?;
                }
                (
                    Shade::Path {
                        path,
                        rect: rectangle,
                        ..
                    },
                    SourceGradientShade::Path {
                        source_ordinal,
                        path: p,
                        fill_to_rect,
                    },
                ) => {
                    let at = budget.at(origin, *source_ordinal)?;
                    set(path, p, &at, budget)?;
                    if let Some(v) = fill_to_rect {
                        InheritedRect::merge(rectangle, v, origin, budget)?;
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
    pub fn finish(self, budget: &mut Budget<'_>) -> Result<EffectiveGradientFill, PptxError> {
        // Account bounded synthesized slots too, including two default stops.
        budget.step()?;
        budget.values(24)?;
        budget.bytes(32)?;
        let shade = match self.shade {
            None => EffectiveGradientShade::Linear {
                declared_by: FillOrigin::ProfileDefault {},
                angle: default(0),
                scaled: default(false),
            },
            Some(Shade::Linear {
                origin,
                angle,
                scaled,
            }) => EffectiveGradientShade::Linear {
                declared_by: origin,
                angle: angle.unwrap_or_else(|| default(0)),
                scaled: scaled.unwrap_or_else(|| default(false)),
            },
            Some(Shade::Path { origin, path, rect }) => EffectiveGradientShade::Path {
                declared_by: origin,
                path: path.unwrap_or_else(|| default(NativePathShade::Shape)),
                fill_to_rect: Box::new(
                    rect.map(|r| r.finish("50000"))
                        .unwrap_or_else(|| default_rect("50000")),
                ),
            },
        };
        Ok(EffectiveGradientFill {
            stops: self.stops.unwrap_or_else(|| {
                default(vec![
                    EffectiveGradientStop {
                        position: default(percentage("0")),
                        color: default_color(SourceColorValue::Srgb { rgb: [0, 0, 0] }),
                    },
                    EffectiveGradientStop {
                        position: default(percentage("100000")),
                        color: default_color(SourceColorValue::Srgb {
                            rgb: [255, 255, 255],
                        }),
                    },
                ])
            }),
            shade,
            tile_rect: self.tile_rect.unwrap_or_else(|| default_rect("0")),
            flip: self.flip.unwrap_or_else(|| default(NativeTileFlip::None)),
            rotate_with_shape: self.rotate.unwrap_or_else(|| default(true)),
        })
    }
}
