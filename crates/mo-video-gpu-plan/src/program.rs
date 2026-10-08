use crate::PaintParameters;
use mo_video_compile::VideoFramePlan;
use mo_video_render::{
    CompositionBackend, FrameRenderOptions, LinearSource, Paint, PaintSource, PixelLimits,
    RenderError, Result, VisualResources, render_with_options,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Immutable host image capability, already or subsequently normalized into the
/// core linear BT.2020/D65 premultiplied profile. The ID grants no file access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuImage {
    id: u32,
    width: u32,
    height: u32,
    has_hdr: bool,
}
impl GpuImage {
    pub fn new(id: u32, width: u32, height: u32, has_hdr: bool) -> Result<Self> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(RenderError::Invalid("GPU image extent"));
        }
        Ok(Self {
            id,
            width,
            height,
            has_hdr,
        })
    }
    pub fn id(&self) -> u32 {
        self.id
    }
}
impl LinearSource for GpuImage {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn has_hdr(&self) -> bool {
        self.has_hdr
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ProgramLimits {
    /// Composition buffers + normalized input buffers + uniform bytes. Opaque
    /// codec/external texture allocations and output presentation are separate.
    pub max_allocated_bytes: u64,
    pub max_commands: usize,
    pub max_images: usize,
}
impl Default for ProgramLimits {
    fn default() -> Self {
        Self {
            max_allocated_bytes: 512 << 20,
            max_commands: 8192,
            max_images: 64,
        }
    }
}
impl ProgramLimits {
    pub(crate) fn validate(self) -> Result<()> {
        if self.max_allocated_bytes < 16
            || self.max_allocated_bytes > 1 << 30
            || self.max_commands == 0
            || self.max_commands > 200_000
            || self.max_images == 0
            || self.max_images > 4096
        {
            return Err(RenderError::Invalid("GPU program limits"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GpuSource {
    Solid,
    Surface { id: u32 },
    Image { id: u32 },
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GpuCommand {
    /// Allocate a zeroed RGBA32F buffer. No write may reach the host queue until
    /// the complete program, capabilities and GPU validation have succeeded.
    Create { id: u32, width: u32, height: u32 },
    Paint {
        target: u32,
        source: GpuSource,
        parameters: [u32; 28],
    },
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuProgram {
    pub profile: &'static str,
    pub commands: Vec<GpuCommand>,
    pub images: Vec<GpuImage>,
    pub output: u32,
    pub width: u32,
    pub height: u32,
    pub has_hdr: bool,
    pub allocated_bytes: u64,
    pub pixel_visits: usize,
}
struct Compiler {
    commands: Vec<GpuCommand>,
    images: BTreeMap<u32, GpuImage>,
    next_surface: u32,
    allocated: u64,
    limits: ProgramLimits,
}
impl Compiler {
    fn admit(&mut self, bytes: u64) -> Result<()> {
        self.allocated = self
            .allocated
            .checked_add(bytes)
            .filter(|n| *n <= self.limits.max_allocated_bytes)
            .ok_or(RenderError::Budget)?;
        if self.commands.len() >= self.limits.max_commands {
            return Err(RenderError::Budget);
        }
        Ok(())
    }
}
impl CompositionBackend for Compiler {
    type Surface = u32;
    type Image = GpuImage;
    fn create_surface(
        &mut self,
        size: (u32, u32),
        _: PixelLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<u32> {
        if check() {
            return Err(RenderError::Cancelled);
        }
        self.admit(u64::from(size.0) * u64::from(size.1) * 16)?;
        let id = self.next_surface;
        self.next_surface = id.checked_add(1).ok_or(RenderError::Budget)?;
        self.commands.push(GpuCommand::Create {
            id,
            width: size.0,
            height: size.1,
        });
        Ok(id)
    }
    fn paint(
        &mut self,
        target: &mut u32,
        paint: Paint<'_, u32, GpuImage>,
        check: &dyn Fn() -> bool,
    ) -> Result<()> {
        if check() {
            return Err(RenderError::Cancelled);
        }
        let (source, solid) = match paint.source {
            PaintSource::Solid(color) => (GpuSource::Solid, Some(color)),
            PaintSource::Surface(id) => (GpuSource::Surface { id: *id }, None),
            PaintSource::Image(image) => {
                if let Some(previous) = self.images.get(&image.id) {
                    if previous != image {
                        return Err(RenderError::Invalid("GPU image identity reused"));
                    }
                } else {
                    if self.images.len() >= self.limits.max_images {
                        return Err(RenderError::Budget);
                    }
                    self.admit(u64::from(image.width) * u64::from(image.height) * 16)?;
                    self.images.insert(image.id, *image);
                }
                (GpuSource::Image { id: image.id }, None)
            }
        };
        let parameters = PaintParameters::new(
            paint.target_size,
            paint.source_size,
            paint.coordinates,
            paint.opacity,
            paint.sampling,
            solid,
        )?
        .words();
        self.admit(112)?;
        self.commands.push(GpuCommand::Paint {
            target: *target,
            source,
            parameters,
        });
        Ok(())
    }
}
pub fn compile_gpu_program(
    plan: &VideoFramePlan,
    resources: &mut dyn VisualResources<GpuImage>,
    options: FrameRenderOptions,
    limits: ProgramLimits,
    check: &dyn Fn() -> bool,
) -> Result<GpuProgram> {
    limits.validate()?;
    // Shared solid placeholder. Uniform/source/target allocations are included
    // exactly once even when the same source is painted repeatedly.
    let mut compiler = Compiler {
        commands: vec![],
        images: BTreeMap::new(),
        next_surface: 0,
        allocated: 16,
        limits,
    };
    let result = render_with_options(plan, resources, options, check, &mut compiler)?;
    Ok(GpuProgram {
        profile: crate::PROFILE,
        commands: compiler.commands,
        images: compiler.images.into_values().collect(),
        output: result.surface,
        width: result.width,
        height: result.height,
        has_hdr: result.has_hdr,
        allocated_bytes: compiler.allocated,
        pixel_visits: result.stats.pixel_visits,
    })
}
