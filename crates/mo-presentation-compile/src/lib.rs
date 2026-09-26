//! Presentation author semantics to evaluated geometry. The placement stage is
//! shared by later painting/hit-testing; it does not resolve text, images or paint.
mod angle;
mod coordinate_budget;
pub mod incremental;
mod interval;
mod interval_extended;
pub mod native_paths;
mod page;
mod page_paint;
mod page_types;
mod path_scene;
#[cfg(test)]
mod path_scene_tests;
mod placement;
mod placement_core;
pub mod playback;
pub mod radial_layout;
mod shape_paths;
pub mod source_frame;
pub mod source_image_layout;
pub mod source_image_paint;
mod source_number;
pub mod source_page;
pub mod source_placement;
pub mod source_playback;
pub mod source_resource_page;
pub mod source_text;
pub mod source_text_page;
mod trig;
mod types;
pub use page::{compile_page, render_page};
pub use page_types::*;
pub use placement::page_placements;
pub use types::*;
pub const PROFILE: &str = "drawingml-sector-scale-reflection-q96-enclosure-v2-draft";
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("invalid document")]
    Document(mo_presentation_model::ValidationReport),
    #[error("invalid placement input: {0}")]
    Invalid(&'static str),
    #[error("placement limit exceeded: {0}")]
    Limit(&'static str),
    #[error("placement outside Q32 coordinate range")]
    Range,
    #[error("placement cancelled")]
    Cancelled,
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), CompileError> {
    if check() {
        Err(CompileError::Cancelled)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod page_tests;
