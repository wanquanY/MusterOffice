//! Native evaluated DrawingML paths -> local shape Q32 EMU paths. Original
//! declarations and guide arithmetic stay owned by the source evaluator.
mod math;
mod path;
#[cfg(test)]
mod tests;
mod types;
use crate::interval::Interval as I;
use mo_pptx::source::geometry::evaluate::{EvaluatedGeometry, GeometryOrigin};
pub use types::*;

struct Work<'a> {
    limits: NativePathLimits,
    check: &'a dyn Fn() -> bool,
    steps: u32,
    paths: u32,
    commands: u32,
    segments: u32,
    origin: GeometryOrigin,
}
impl Work<'_> {
    fn step(&mut self) -> Result<(), NativePathError> {
        if (self.check)() {
            return Err(NativePathError::Cancelled);
        }
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(NativePathError::Limit("work"))?;
        if self.steps > self.limits.max_steps {
            return Err(NativePathError::Limit("work"));
        }
        Ok(())
    }
    fn issue(&self, issue: NativePathIssue) -> NativePathError {
        NativePathError::Geometry {
            origin: self.origin,
            issue,
        }
    }
    fn numeric(&self) -> NativePathError {
        self.issue(NativePathIssue::NumericRange)
    }
    fn precision(&self) -> NativePathError {
        self.issue(NativePathIssue::PrecisionExceeded)
    }
}
/// A compiler instance owns one shared budget. Repeated objects and failed
/// objects consume it; callers must not create a fresh instance per query item.
pub struct NativePathCompiler<'a> {
    options: NativePathOptions,
    work: Work<'a>,
}
impl<'a> NativePathCompiler<'a> {
    pub fn new(
        options: NativePathOptions,
        limits: NativePathLimits,
        check: &'a dyn Fn() -> bool,
    ) -> Result<Self, NativePathError> {
        if options.coordinate_tolerance.raw() <= 0 {
            return Err(NativePathError::Options);
        }
        Ok(Self {
            options,
            work: Work {
                limits,
                check,
                steps: 0,
                paths: 0,
                commands: 0,
                segments: 0,
                origin: GeometryOrigin::document(0),
            },
        })
    }
    pub fn compile(
        &mut self,
        geometry: &EvaluatedGeometry,
    ) -> Result<Vec<CompiledNativePath>, NativePathError> {
        self.compile_with_tolerance(geometry, self.options.coordinate_tolerance)
    }
    /// Request tighter geometry for a dependent calculation (for example path
    /// bounds used by a gradient). Retains the same page-wide work budget.
    pub fn compile_with_tolerance(
        &mut self,
        geometry: &EvaluatedGeometry,
        tolerance: mo_geometry::Fixed,
    ) -> Result<Vec<CompiledNativePath>, NativePathError> {
        if tolerance.raw() <= 0 || tolerance > self.options.coordinate_tolerance {
            return Err(NativePathError::Options);
        }
        self.work.origin = GeometryOrigin::document(geometry.source_ordinal);
        self.work.step()?;
        let size = geometry.extent.value;
        if [size.width.get(), size.height.get()]
            .iter()
            .any(|v| !(0..=27_273_042_316_900).contains(v))
        {
            return Err(self.work.issue(NativePathIssue::InvalidExtent));
        }
        let mut paths = Vec::new();
        for path in &geometry.paths {
            self.work.origin = path.origin;
            self.work.step()?;
            self.work.paths = self
                .work
                .paths
                .checked_add(1)
                .ok_or(NativePathError::Limit("paths"))?;
            if self.work.paths > self.work.limits.max_paths {
                return Err(NativePathError::Limit("paths"));
            }
            let axis =
                |extent: i64, dimension: Option<mo_common::Emu>| -> Result<I, NativePathError> {
                    match dimension {
                        None => Ok(I::integer(1)),
                        Some(value) if value.get() == 0 => {
                            Err(self.work.issue(NativePathIssue::ZeroPathExtent))
                        }
                        Some(value) if value.get() < 0 => {
                            Err(self.work.issue(NativePathIssue::InvalidExtent))
                        }
                        Some(value) => Ok(I::ratio(extent, value.get())),
                    }
                };
            let scale = [
                axis(size.width.get(), path.width)?,
                axis(size.height.get(), path.height)?,
            ];
            paths.push(path::compile(path, scale, tolerance, &mut self.work)?);
        }
        Ok(paths)
    }
}
