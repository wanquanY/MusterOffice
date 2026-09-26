use super::*;

/// srcRect and fillToRect participate in property inheritance. In contrast,
/// tileRect and stretch/fillRect use the schema-default rectangle in rect().
pub(super) struct InheritedRect {
    origin: FillOrigin,
    left: Option<FillValue<NativePercentage>>,
    top: Option<FillValue<NativePercentage>>,
    right: Option<FillValue<NativePercentage>>,
    bottom: Option<FillValue<NativePercentage>>,
}
impl InheritedRect {
    pub fn merge(
        slot: &mut Option<Box<Self>>,
        value: &SourceFillRect,
        origin: &FillOrigin,
        budget: &mut Budget<'_>,
    ) -> Result<(), PptxError> {
        let at = budget.at(origin, value.source_ordinal)?;
        if slot.is_none() {
            *slot = Some(Box::new(Self {
                origin: budget.origin(&at)?,
                left: None,
                top: None,
                right: None,
                bottom: None,
            }));
        }
        let rect = slot.as_mut().expect("selected rectangle");
        set(&mut rect.left, &value.left, &at, budget)?;
        set(&mut rect.top, &value.top, &at, budget)?;
        set(&mut rect.right, &value.right, &at, budget)?;
        set(&mut rect.bottom, &value.bottom, &at, budget)
    }
    pub fn complete(&self) -> bool {
        self.left.is_some() && self.top.is_some() && self.right.is_some() && self.bottom.is_some()
    }
    pub fn finish(self, fallback: &str) -> EffectiveFillRect {
        let edge = || default(percentage(fallback));
        EffectiveFillRect {
            declared_by: self.origin,
            left: self.left.unwrap_or_else(edge),
            top: self.top.unwrap_or_else(edge),
            right: self.right.unwrap_or_else(edge),
            bottom: self.bottom.unwrap_or_else(edge),
        }
    }
}
