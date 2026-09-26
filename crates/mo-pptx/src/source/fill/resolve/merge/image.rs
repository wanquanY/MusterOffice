use super::*;

#[derive(Default)]
pub(super) struct Image {
    embed: Option<FillValue<String>>,
    link: Option<FillValue<String>>,
    compression: Option<FillValue<NativeBlipCompression>>,
    source_rect: Option<Box<InheritedRect>>,
    mode: Option<Mode>,
    dpi: Option<FillValue<u32>>,
    rotate: Option<FillValue<bool>>,
}
enum Mode {
    Tile(FillOrigin, Box<Tile>),
    Stretch(FillOrigin, Option<Box<EffectiveFillRect>>),
}
#[derive(Default)]
struct Tile {
    tx: Option<FillValue<NativeCoordinate>>,
    ty: Option<FillValue<NativeCoordinate>>,
    sx: Option<FillValue<NativePercentage>>,
    sy: Option<FillValue<NativePercentage>>,
    flip: Option<FillValue<NativeTileFlip>>,
    alignment: Option<FillValue<NativeFillAlignment>>,
}
impl Image {
    pub fn complete(&self) -> bool {
        self.embed.is_some()
            && self.link.is_some()
            && self.compression.is_some()
            && self.source_rect.as_ref().is_some_and(|r| r.complete())
            && self.dpi.is_some()
            && self.rotate.is_some()
            && match &self.mode {
                None => false,
                Some(Mode::Stretch(_, r)) => r.is_some(),
                Some(Mode::Tile(_, t)) => t.complete(),
            }
    }
    pub fn merge(
        &mut self,
        v: &SourceImageFill,
        origin: &FillOrigin,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        if let Some(id) = v.blip.as_ref().and_then(|b| b.effect_nodes.first()) {
            return Err(FillUnresolved::EffectEvaluationRequired {
                origin: budget.at(origin, *id)?,
            }
            .into());
        }
        set(&mut self.dpi, &v.dpi, origin, budget)?;
        set(&mut self.rotate, &v.rotate_with_shape, origin, budget)?;
        if let Some(blip) = &v.blip {
            let at = budget.at(origin, blip.source_ordinal)?;
            set(&mut self.embed, &blip.embed, &at, budget)?;
            set(&mut self.link, &blip.link, &at, budget)?;
            set(&mut self.compression, &blip.compression, &at, budget)?;
        }
        if let Some(r) = &v.source_rect {
            InheritedRect::merge(&mut self.source_rect, r, origin, budget)?;
        }
        if let Some(mode) = &v.mode {
            if self.mode.is_none() {
                self.mode = Some(match mode {
                    SourceImageFillMode::Tile(t) => {
                        Mode::Tile(budget.at(origin, t.source_ordinal)?, Box::default())
                    }
                    SourceImageFillMode::Stretch { source_ordinal, .. } => {
                        Mode::Stretch(budget.at(origin, *source_ordinal)?, None)
                    }
                });
            }
            match (self.mode.as_mut().expect("mode selected"), mode) {
                (Mode::Tile(_, t), SourceImageFillMode::Tile(v)) => {
                    let at = budget.at(origin, v.source_ordinal)?;
                    t.merge(v, &at, budget)?;
                }
                (
                    Mode::Stretch(_, r),
                    SourceImageFillMode::Stretch {
                        fill_rect: Some(v), ..
                    },
                ) if r.is_none() => *r = Some(Box::new(rect(v, origin, budget)?)),
                _ => (),
            }
        }
        Ok(())
    }
    pub fn finish(
        self,
        origin: &FillOrigin,
        budget: &mut Budget<'_>,
    ) -> Result<EffectiveImageFill, Failure> {
        budget.step()?;
        budget.values(28)?;
        budget.bytes(32)?;
        let embed = self.embed.unwrap_or_else(|| default(String::new()));
        let link = self.link.unwrap_or_else(|| default(String::new()));
        if embed.value.is_empty() && link.value.is_empty() {
            return Err(FillUnresolved::MissingImage {
                origin: budget.origin(origin)?,
            }
            .into());
        }
        let mode = match self.mode {
            None => EffectiveImageMode::Tile {
                declared_by: FillOrigin::ProfileDefault {},
                tile: Tile::default().finish(),
            },
            Some(Mode::Tile(declared_by, t)) => EffectiveImageMode::Tile {
                declared_by,
                tile: t.finish(),
            },
            Some(Mode::Stretch(declared_by, r)) => EffectiveImageMode::Stretch {
                declared_by,
                fill_rect: r.map(|r| *r).unwrap_or_else(|| default_rect("0")),
            },
        };
        Ok(EffectiveImageFill {
            embed,
            link,
            compression: self
                .compression
                .unwrap_or_else(|| default(NativeBlipCompression::None)),
            source_rect: self
                .source_rect
                .map(|r| r.finish("0"))
                .unwrap_or_else(|| default_rect("0")),
            mode,
            dpi: self.dpi.unwrap_or_else(|| default(0)),
            rotate_with_shape: self.rotate.unwrap_or_else(|| default(true)),
        })
    }
}
impl Tile {
    fn complete(&self) -> bool {
        self.tx.is_some()
            && self.ty.is_some()
            && self.sx.is_some()
            && self.sy.is_some()
            && self.flip.is_some()
            && self.alignment.is_some()
    }
    fn merge(
        &mut self,
        v: &SourceFillTile,
        origin: &FillOrigin,
        budget: &mut Budget<'_>,
    ) -> Result<(), PptxError> {
        set(&mut self.tx, &v.translate_x, origin, budget)?;
        set(&mut self.ty, &v.translate_y, origin, budget)?;
        set(&mut self.sx, &v.scale_x, origin, budget)?;
        set(&mut self.sy, &v.scale_y, origin, budget)?;
        set(&mut self.flip, &v.flip, origin, budget)?;
        set(&mut self.alignment, &v.alignment, origin, budget)
    }
    fn finish(self) -> EffectiveFillTile {
        let zero = || default("0".to_owned().try_into().expect("native zero coordinate"));
        EffectiveFillTile {
            translate_x: self.tx.unwrap_or_else(zero),
            translate_y: self.ty.unwrap_or_else(zero),
            scale_x: self.sx.unwrap_or_else(|| default(percentage("100000"))),
            scale_y: self.sy.unwrap_or_else(|| default(percentage("100000"))),
            flip: self.flip.unwrap_or_else(|| default(NativeTileFlip::None)),
            alignment: self
                .alignment
                .unwrap_or_else(|| default(NativeFillAlignment::TopLeft)),
        }
    }
}
