mod support;
#[path = "hardware/viewport.rs"]
mod viewport;
use mo_video_gpu::*;
use mo_video_render::*;
use serde_json::json;
use std::{cell::Cell, path::PathBuf, sync::mpsc, time::Duration};
use support::*;

fn readback(device: &wgpu::Device, queue: &wgpu::Queue, encoded: EncodedFrame) -> LinearImage {
    let out = encoded.output;
    let bytes = u64::from(out.width) * u64::from(out.height) * 16;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test-readback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut copy = device.create_command_encoder(&Default::default());
    copy.copy_buffer_to_buffer(&out.surface, 0, &buffer, 0, bytes);
    let submission = queue.submit([encoded.commands, copy.finish()]);
    let (tx, rx) = mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
    let view = buffer.slice(..).get_mapped_range();
    let pixels = view
        .chunks_exact(16)
        .map(|p| {
            std::array::from_fn(|i| f32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap()))
        })
        .collect();
    drop(view);
    buffer.unmap();
    LinearImage::from_pixels(
        out.width,
        out.height,
        pixels,
        out.has_hdr,
        PixelLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn compare(cpu: &LinearImage, gpu: &LinearImage) -> (f64, f64, u8) {
    assert_eq!(
        (cpu.width(), cpu.height(), cpu.has_hdr()),
        (gpu.width(), gpu.height(), gpu.has_hdr())
    );
    let mut maximum: f64 = 0.;
    let mut squared = 0.;
    for (a, b) in cpu
        .pixels()
        .iter()
        .flatten()
        .zip(gpu.pixels().iter().flatten())
    {
        let error = (f64::from(*a) - f64::from(*b)).abs();
        maximum = maximum.max(error);
        squared += error * error;
    }
    let rms = (squared / (cpu.pixels().len() * 4) as f64).sqrt();
    let output = OutputOptions {
        tone_map: cpu.has_hdr().then_some(ToneMap::ReinhardLuminanceV1 {
            source_peak_nits: 1000.,
        }),
        alpha: OutputAlpha::Straight,
        max_bytes: 1 << 24,
    };
    let cpu8 = cpu.to_srgb8(output, &|| false).unwrap();
    let gpu8 = gpu.to_srgb8(output, &|| false).unwrap();
    let code_error = cpu8
        .rgba
        .iter()
        .zip(&gpu8.rgba)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(
        maximum <= 2e-5 && rms <= 2e-6 && code_error <= 1,
        "linear max {maximum}, RMS {rms}, RGBA8 {code_error}"
    );
    (maximum, rms, code_error)
}

#[test]
#[ignore = "requires an actual hardware GPU; execute the explicit video GPU gate"]
fn actual_gpu_matches_reference_and_discards_cancelled_candidates() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .expect("hardware GPU required");
    let info = adapter.get_info();
    assert_ne!(
        info.device_type,
        wgpu::DeviceType::Cpu,
        "software adapter cannot satisfy hardware evidence"
    );
    assert_ne!(info.backend, wgpu::Backend::Noop);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("video-gpu-check"),
        ..Default::default()
    }))
    .unwrap();
    let compositor = pollster::block_on(GpuCompositor::new(device.clone())).unwrap();
    let (mut resources, _) = source(image());
    let mut records = vec![];
    for (name, plan) in cases() {
        for sampling in [Sampling::Nearest, Sampling::Bilinear] {
            eprintln!("compare {name} {sampling:?}");
            let cpu = render_frame(
                &plan,
                &mut resources,
                sampling,
                RenderLimits::default(),
                &|| false,
            )
            .unwrap();
            let candidate = pollster::block_on(compositor.encode(
                &plan,
                &mut resources,
                sampling,
                RenderLimits::default(),
                GpuLimits::default(),
                &|| false,
            ))
            .unwrap();
            assert_eq!(candidate.output.stats.nodes, cpu.stats.nodes);
            assert_eq!(candidate.output.stats.pixel_visits, cpu.stats.pixel_visits);
            let stats = candidate.gpu_stats;
            let gpu = readback(&device, &queue, candidate);
            let (max, rms, code) = compare(&cpu.image, &gpu);
            if name == "group-opacity" {
                let out = gpu
                    .to_srgb8(
                        OutputOptions {
                            tone_map: None,
                            alpha: OutputAlpha::Straight,
                            max_bytes: 1 << 20,
                        },
                        &|| false,
                    )
                    .unwrap();
                assert!(out.rgba.chunks_exact(4).all(|p| p == [188, 188, 255, 255]));
            }
            records.push(json!({"case":name,"sampling":format!("{sampling:?}"),"maxLinearError":max,"rmsLinearError":rms,"maxRgba8Error":code,"allocatedBytes":stats.allocated_bytes,"uploadBytes":stats.upload_bytes,"dispatches":stats.dispatches}));
        }
    }
    let plan = cases()
        .into_iter()
        .find(|(name, _)| name == "transform--47000")
        .unwrap()
        .1;
    let calls = Cell::new(0);
    drop(
        pollster::block_on(compositor.encode(
            &plan,
            &mut resources,
            Sampling::Bilinear,
            RenderLimits::default(),
            GpuLimits::default(),
            &|| {
                calls.set(calls.get() + 1);
                false
            },
        ))
        .unwrap(),
    );
    let cancellation_points = calls.get();
    for cancel_at in 1..=cancellation_points {
        calls.set(0);
        let r = pollster::block_on(compositor.encode(
            &plan,
            &mut resources,
            Sampling::Bilinear,
            RenderLimits::default(),
            GpuLimits::default(),
            &|| {
                calls.set(calls.get() + 1);
                calls.get() >= cancel_at
            },
        ));
        assert!(
            matches!(r, Err(RenderError::Cancelled)),
            "cancel at {cancel_at}"
        );
    }
    for limits in [
        GpuLimits {
            max_allocated_bytes: 16,
            max_upload_bytes: 16,
            ..Default::default()
        },
        GpuLimits {
            max_upload_bytes: 0,
            ..Default::default()
        },
        GpuLimits {
            max_dispatches: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            pollster::block_on(compositor.encode(
                &plan,
                &mut resources,
                Sampling::Bilinear,
                RenderLimits::default(),
                limits,
                &|| false
            )),
            Err(RenderError::Budget)
        ));
    }
    // Device remains usable after discarded encoders/scopes; no stale candidate state.
    struct PanicResource;
    impl VisualResources for PanicResource {
        fn media(
            &mut self,
            _: &mo_video_compile::MediaBinding,
            _: &mo_video_model::TimeSample,
            _: &dyn Fn() -> bool,
        ) -> Result<ResolvedMedia> {
            panic!("test host callback panic");
        }
    }
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pollster::block_on(compositor.encode(
                &plan,
                &mut PanicResource,
                Sampling::Bilinear,
                RenderLimits::default(),
                GpuLimits::default(),
                &|| false,
            ))
        }))
        .is_err()
    );
    let recovered = pollster::block_on(compositor.encode(
        &plan,
        &mut resources,
        Sampling::Bilinear,
        RenderLimits::default(),
        GpuLimits::default(),
        &|| false,
    ))
    .unwrap();
    let gpu = readback(&device, &queue, recovered);
    let cpu = render_frame(
        &plan,
        &mut resources,
        Sampling::Bilinear,
        RenderLimits::default(),
        &|| false,
    )
    .unwrap();
    compare(&cpu.image, &gpu);

    let (mut hdr_resources, mut hdr_visual) = source(
        LinearImage::from_pixels(
            2,
            2,
            vec![
                [5., 2., -0.1, 1.],
                [2., -0.2, 1., 0.5],
                [0.; 4],
                [3., 4., 5., 1.],
            ],
            true,
            PixelLimits::default(),
            &|| false,
        )
        .unwrap(),
    );
    hdr_visual.effects.push(transform(23000, 850_000));
    let hdr_plan = support::plan(vec![hdr_visual], 64, 64);
    let cpu = render_frame(
        &hdr_plan,
        &mut hdr_resources,
        Sampling::Bilinear,
        RenderLimits::default(),
        &|| false,
    )
    .unwrap();
    let encoded = pollster::block_on(compositor.encode(
        &hdr_plan,
        &mut hdr_resources,
        Sampling::Bilinear,
        RenderLimits::default(),
        GpuLimits::default(),
        &|| false,
    ))
    .unwrap();
    let gpu = readback(&device, &queue, encoded);
    let (max, rms, code) = compare(&cpu.image, &gpu);
    records.push(
        json!({"case":"hdr-linear","maxLinearError":max,"rmsLinearError":rms,"maxRgba8Error":code}),
    );
    let mut real_frames = 0;
    if let Some(path) = std::env::var_os("MO_VIDEO_GPU_LINEAR_INPUT") {
        let data = std::fs::read(path).unwrap();
        let frame_bytes = 128 * 96 * 3 * 4;
        assert_eq!(data.len(), 77 * frame_bytes);
        for index in [0, 17, 40, 76] {
            let pixels = data[index * frame_bytes..(index + 1) * frame_bytes]
                .chunks_exact(12)
                .map(|p| {
                    [
                        f32::from_le_bytes(p[0..4].try_into().unwrap()),
                        f32::from_le_bytes(p[4..8].try_into().unwrap()),
                        f32::from_le_bytes(p[8..12].try_into().unwrap()),
                        1.,
                    ]
                })
                .collect();
            let (mut resources, mut visual) = source(
                LinearImage::from_pixels(128, 96, pixels, false, PixelLimits::default(), &|| false)
                    .unwrap(),
            );
            visual.effects.push(transform(15000, 750_000));
            let p = support::plan(vec![solid("bg", [15, 23, 42, 255]), visual], 160, 120);
            let cpu = render_frame(
                &p,
                &mut resources,
                Sampling::Bilinear,
                RenderLimits::default(),
                &|| false,
            )
            .unwrap();
            let candidate = pollster::block_on(compositor.encode(
                &p,
                &mut resources,
                Sampling::Bilinear,
                RenderLimits::default(),
                GpuLimits::default(),
                &|| false,
            ))
            .unwrap();
            let gpu = readback(&device, &queue, candidate);
            let (max, rms, code) = compare(&cpu.image, &gpu);
            records.push(json!({"case":format!("real-frame-{index}"),"maxLinearError":max,"rmsLinearError":rms,"maxRgba8Error":code}));
            real_frames += 1;
        }
    }
    let report = json!({"format":"musteroffice.video-gpu-check/1","status":"passed","adapter":info.name,"backend":format!("{:?}",info.backend),"deviceType":format!("{:?}",info.device_type),"driver":info.driver,"driverInfo":info.driver_info,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"profile":mo_video_gpu::PROFILE,"realFrames":real_frames,"cancellationPoints":cancellation_points,"cases":records,"scope":"Actual hardware compute/readback versus CPU linear pixels. Not browser, device-loss recovery, zero-copy, real-time performance or product acceptance."});
    if let Some(path) = std::env::var_os("MO_VIDEO_GPU_REPORT") {
        let mut options = std::fs::OpenOptions::new();
        use std::io::Write;
        let mut file = options
            .write(true)
            .create_new(true)
            .open(PathBuf::from(path))
            .unwrap();
        file.write_all(serde_json::to_string_pretty(&report).unwrap().as_bytes())
            .unwrap();
    }
    println!("{report}");
}
