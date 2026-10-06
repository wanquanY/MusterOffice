use crate::execution::Execution;
use mo_video_compile::VideoFramePlan;
use mo_video_render::{
    CompositionResult, RenderError, RenderLimits, Result, Sampling, VisualResources,
    render_with_backend,
};
use wgpu::{Device, ErrorFilter, ErrorScopeGuard, util::DeviceExt};

pub const SHADER: &str = include_str!("compose.wgsl");
pub const PROFILE: &str = "musteroffice.video.linear2020-premul-gpu-f32-v1-draft";
/// Explicit numeric capability of this GPU profile; no silent saturation.
pub const MAX_ABS_COMPONENT: f32 = 1e12;

#[derive(Debug, Clone, Copy)]
pub struct GpuLimits {
    /// Sum of all buffers allocated for a single encoded frame, including
    /// resources retained by its command buffer. Not driver memory or RSS.
    pub max_allocated_bytes: u64,
    pub max_upload_bytes: u64,
    pub max_dispatches: usize,
}
impl Default for GpuLimits {
    fn default() -> Self {
        Self {
            max_allocated_bytes: 512 << 20,
            max_upload_bytes: 256 << 20,
            max_dispatches: 4096,
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct GpuStats {
    pub allocated_bytes: u64,
    pub upload_bytes: u64,
    pub dispatches: usize,
}
/// Unsubmitted commands, not a completed/published frame. The host submits on
/// this device's queue and owns completion, device loss, cancellation and reuse.
/// No CPU readback is necessary for further GPU processing/presentation.
pub struct EncodedFrame {
    pub commands: wgpu::CommandBuffer,
    pub output: CompositionResult<wgpu::Buffer>,
    pub gpu_stats: GpuStats,
}
pub struct GpuCompositor {
    pub(crate) device: Device,
    pub(crate) pipeline: wgpu::ComputePipeline,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) dummy: wgpu::Buffer,
}
impl GpuCompositor {
    /// The host selects adapter, backend and requested device limits. This does
    /// not fall back to a no-op or CPU device, or mutate global error handlers.
    pub async fn new(device: Device) -> Result<Self> {
        if cfg!(target_endian = "big") {
            return Err(RenderError::Unsupported("GPU upload byte order"));
        }
        let errors = ErrorScopes::new(&device);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PROFILE),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(PROFILE),
            layout: None,
            module: &module,
            entry_point: Some("compose"),
            compilation_options: Default::default(),
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let dummy = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("video-solid-placeholder"),
            contents: &[0; 16],
            usage: wgpu::BufferUsages::STORAGE,
        });
        errors.finish().await?;
        Ok(Self {
            device,
            pipeline,
            layout,
            dummy,
        })
    }
    /// All work remains in a fresh encoder until the complete plan, resources,
    /// budgets and GPU validation succeed. A cancellation returns no commands.
    pub async fn encode(
        &self,
        plan: &VideoFramePlan,
        resources: &mut dyn VisualResources,
        sampling: Sampling,
        render_limits: RenderLimits,
        gpu_limits: GpuLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<EncodedFrame> {
        cancelled(check)?;
        if gpu_limits.max_allocated_bytes == 0
            || gpu_limits.max_allocated_bytes > 1 << 30
            || gpu_limits.max_upload_bytes > gpu_limits.max_allocated_bytes
            || gpu_limits.max_dispatches == 0
            || gpu_limits.max_dispatches > 100_000
        {
            return Err(RenderError::Invalid("GPU limits"));
        }
        let errors = ErrorScopes::new(&self.device);
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("video-frame-candidate"),
            });
        let mut execution = Execution {
            renderer: self,
            encoder,
            limits: gpu_limits,
            stats: GpuStats::default(),
        };
        let result = render_with_backend(
            plan,
            resources,
            sampling,
            render_limits,
            check,
            &mut execution,
        );
        // Even on a pure-input failure pop/observe scopes; encoded work is dropped.
        let commands = if result.is_ok() {
            Some(execution.encoder.finish())
        } else {
            None
        };
        let validation = errors.finish().await;
        cancelled(check)?;
        let output = result?;
        validation?;
        Ok(EncodedFrame {
            commands: commands.expect("successful encoding has commands"),
            output,
            gpu_stats: execution.stats,
        })
    }
}
pub(crate) fn cancelled(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(RenderError::Cancelled)
    } else {
        Ok(())
    }
}
struct ErrorScopes {
    // Rust drops fields in declaration order, including unwinding through a
    // host resource callback. Keep the native scope stack in LIFO order.
    validation: ErrorScopeGuard,
    internal: ErrorScopeGuard,
    oom: ErrorScopeGuard,
}
impl ErrorScopes {
    fn new(device: &Device) -> Self {
        let oom = device.push_error_scope(ErrorFilter::OutOfMemory);
        let internal = device.push_error_scope(ErrorFilter::Internal);
        let validation = device.push_error_scope(ErrorFilter::Validation);
        Self {
            validation,
            internal,
            oom,
        }
    }
    async fn finish(self) -> Result<()> {
        // Pop every thread-local scope before the first yield.
        let v = self.validation.pop();
        let i = self.internal.pop();
        let o = self.oom.pop();
        let errors = [v.await, i.await, o.await];
        if let Some(error) = errors.into_iter().flatten().next() {
            Err(RenderError::Resource(format!("GPU validation: {error}")))
        } else {
            Ok(())
        }
    }
}
