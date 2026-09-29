//! Native editorial identity for the implemented presentation behaviors. Presets
//! describe the effect in presentation editors; payloads still own all values.
use mo_timeline::{Effect, PresentationPreset};

pub const fn native_preset(preset: PresentationPreset) -> (u32, &'static str) {
    match preset {
        PresentationPreset::Appear => (1, "entr"),
        PresentationPreset::Disappear => (1, "exit"),
        PresentationPreset::Spin => (8, "emph"),
        PresentationPreset::GrowShrink => (6, "emph"),
        PresentationPreset::CustomMotion => (0, "path"),
        PresentationPreset::FadeIn => (10, "entr"),
        PresentationPreset::FadeOut => (10, "exit"),
    }
}
pub(super) fn presentation_preset(id: u32, class: &str) -> Option<PresentationPreset> {
    [
        PresentationPreset::Appear,
        PresentationPreset::Disappear,
        PresentationPreset::Spin,
        PresentationPreset::GrowShrink,
        PresentationPreset::CustomMotion,
        PresentationPreset::FadeIn,
        PresentationPreset::FadeOut,
    ]
    .into_iter()
    .find(|p| native_preset(*p) == (id, class))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformPreset {
    GrowShrink,
    Spin,
    CustomMotion,
}
impl TransformPreset {
    pub fn for_effect(effect: &Effect) -> Option<Self> {
        match effect {
            Effect::Scale { .. } => Some(Self::GrowShrink),
            Effect::Rotation { .. } => Some(Self::Spin),
            Effect::MotionLine { .. } | Effect::MotionPath { .. } => Some(Self::CustomMotion),
            // A property assignment is not an entrance/exit preset. Inventing
            // such a preset would change the slide's initial visibility.
            Effect::SetVisibility { .. } | Effect::Fade { .. } => None,
        }
    }
    pub const fn id(self) -> u32 {
        native_preset(self.presentation()).0
    }
    pub const fn class(self) -> &'static str {
        native_preset(self.presentation()).1
    }
    const fn presentation(self) -> PresentationPreset {
        match self {
            Self::GrowShrink => PresentationPreset::GrowShrink,
            Self::Spin => PresentationPreset::Spin,
            Self::CustomMotion => PresentationPreset::CustomMotion,
        }
    }
    pub(super) fn for_behavior(name: &str) -> Option<Self> {
        match name {
            "animScale" => Some(Self::GrowShrink),
            "animRot" => Some(Self::Spin),
            "animMotion" => Some(Self::CustomMotion),
            _ => None,
        }
    }
}
