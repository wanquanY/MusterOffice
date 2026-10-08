use crate::{GpuCompositor, GpuLimits, GpuStats, MAX_ABS_COMPONENT, encode::cancelled};
use mo_video_render::{CompositionBackend, Paint, PaintSource, PixelLimits, RenderError, Result};
use wgpu::util::DeviceExt;

use mo_video_gpu_plan::PaintParameters;

pub(crate) struct Execution<'a> {
    pub renderer: &'a GpuCompositor,
    pub encoder: wgpu::CommandEncoder,
    pub limits: GpuLimits,
    pub stats: GpuStats,
}
impl Execution<'_> {
    fn admit(&mut self, bytes: u64, upload: bool, storage: bool) -> Result<()> {
        let device = self.renderer.device.limits();
        if bytes > device.max_buffer_size
            || (storage && bytes > device.max_storage_buffer_binding_size)
        {
            return Err(RenderError::Unsupported("GPU buffer extent"));
        }
        let allocated = self
            .stats
            .allocated_bytes
            .checked_add(bytes)
            .ok_or(RenderError::Budget)?;
        let uploaded = self
            .stats
            .upload_bytes
            .checked_add(if upload { bytes } else { 0 })
            .ok_or(RenderError::Budget)?;
        if allocated > self.limits.max_allocated_bytes || uploaded > self.limits.max_upload_bytes {
            return Err(RenderError::Budget);
        }
        self.stats.allocated_bytes = allocated;
        self.stats.upload_bytes = uploaded;
        Ok(())
    }
}
impl CompositionBackend for Execution<'_> {
    type Surface = wgpu::Buffer;
    type Image = std::sync::Arc<mo_video_render::LinearImage>;
    fn create_surface(
        &mut self,
        size: (u32, u32),
        _: PixelLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self::Surface> {
        cancelled(check)?;
        let bytes = u64::from(size.0) * u64::from(size.1) * 16;
        self.admit(bytes, false, true)?;
        let buffer = self.renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("video-linear-surface"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Explicit clear in the candidate encoder; no queue writes before commit.
        self.encoder.clear_buffer(&buffer, 0, None);
        Ok(buffer)
    }
    fn paint(
        &mut self,
        target: &mut Self::Surface,
        paint: Paint<'_, Self::Surface>,
        check: &dyn Fn() -> bool,
    ) -> Result<()> {
        cancelled(check)?;
        if self.stats.dispatches >= self.limits.max_dispatches {
            return Err(RenderError::Budget);
        }
        let (width, height) = paint.target_size;
        let (source_width, source_height) = paint.source_size;
        let dispatch = (width.div_ceil(8), height.div_ceil(8));
        let max_dispatch = self
            .renderer
            .device
            .limits()
            .max_compute_workgroups_per_dimension;
        if dispatch.0 > max_dispatch || dispatch.1 > max_dispatch {
            return Err(RenderError::Unsupported("GPU dispatch extent"));
        }
        let mut solid = [0.; 4];
        let mut is_solid = 0;
        let uploaded;
        let source = match paint.source {
            PaintSource::Solid(p) => {
                solid = p;
                is_solid = 1;
                &self.renderer.dummy
            }
            PaintSource::Surface(buffer) => buffer,
            PaintSource::Image(image) => {
                for chunk in image.pixels().chunks(4096) {
                    cancelled(check)?;
                    if chunk.iter().flatten().any(|v| v.abs() > MAX_ABS_COMPONENT) {
                        return Err(RenderError::Unsupported("GPU linear component range"));
                    }
                }
                let bytes = bytemuck::cast_slice(image.pixels());
                self.admit(bytes.len() as u64, true, true)?;
                uploaded =
                    self.renderer
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("video-linear-source"),
                            contents: bytes,
                            usage: wgpu::BufferUsages::STORAGE,
                        });
                &uploaded
            }
        };
        let parameters = PaintParameters::new(
            (width, height),
            (source_width, source_height),
            paint.coordinates,
            paint.opacity,
            paint.sampling,
            if is_solid == 1 { Some(solid) } else { None },
        )?;
        self.admit(std::mem::size_of::<PaintParameters>() as u64, true, false)?;
        let uniform = self
            .renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("video-paint-parameters"),
                contents: bytemuck::bytes_of(&parameters),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let group = self
            .renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("video-paint-bindings"),
                layout: &self.renderer.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: source.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: target.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            });
        {
            let mut pass = self
                .encoder
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("video-linear-compose"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(&self.renderer.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(dispatch.0, dispatch.1, 1);
        }
        self.stats.dispatches += 1;
        cancelled(check)
    }
}
