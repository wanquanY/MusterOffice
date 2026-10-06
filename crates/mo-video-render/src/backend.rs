//! Backend boundary. All authoring semantics and resource admission stay in the
//! common plan executor; implementations only allocate and paint linear pixels.
use crate::{LinearImage, PixelLimits, RasterCoordinates, RenderStats, Result, Sampling};
pub enum PaintSource<'a, S> {
    Solid([f32; 4]),
    Image(&'a LinearImage),
    Surface(&'a S),
}
pub struct Paint<'a, S> {
    pub source: PaintSource<'a, S>,
    pub target_size: (u32, u32),
    pub source_size: (u32, u32),
    pub coordinates: RasterCoordinates,
    pub opacity: f64,
    pub sampling: Sampling,
}
pub trait CompositionBackend {
    type Surface;
    fn create_surface(
        &mut self,
        size: (u32, u32),
        limits: PixelLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self::Surface>;
    fn paint(
        &mut self,
        target: &mut Self::Surface,
        paint: Paint<'_, Self::Surface>,
        check: &dyn Fn() -> bool,
    ) -> Result<()>;
}
pub struct CompositionResult<S> {
    pub surface: S,
    pub width: u32,
    pub height: u32,
    pub has_hdr: bool,
    pub stats: RenderStats,
}
