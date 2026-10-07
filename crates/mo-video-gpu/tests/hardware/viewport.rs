// Included by hardware.rs so both gates use the same independent readback and
// error metrics. Not a standalone integration test target.
use super::*;

#[test]
#[ignore = "requires an actual hardware GPU; execute the viewport GPU gate"]
fn actual_gpu_uses_the_same_project_to_raster_coordinates() {
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
    assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
    assert_ne!(info.backend, wgpu::Backend::Noop);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("video-viewport-check"),
        ..Default::default()
    }))
    .unwrap();
    let compositor = pollster::block_on(GpuCompositor::new(device.clone())).unwrap();
    let (mut resources, _) = source(image());
    let mut records = vec![];
    for (name, plan) in cases() {
        for (width, height) in [(13, 9), (97, 71), (16, 32), (64, 64)] {
            for sampling in [Sampling::Nearest, Sampling::Bilinear] {
                eprintln!("viewport {name} {width}x{height} {sampling:?}");
                let options = FrameRenderOptions {
                    raster: RasterExtent { width, height },
                    sampling,
                    limits: RenderLimits::default(),
                };
                let cpu =
                    render_frame_with_options(&plan, &mut resources, options, &|| false).unwrap();
                let encoded = pollster::block_on(compositor.encode_with_options(
                    &plan,
                    &mut resources,
                    options,
                    GpuLimits::default(),
                    &|| false,
                ))
                .unwrap();
                assert_eq!(
                    (encoded.output.width, encoded.output.height),
                    (width, height)
                );
                assert_eq!(encoded.output.stats.pixel_visits, cpu.stats.pixel_visits);
                let stats = encoded.gpu_stats;
                let gpu = readback(&device, &queue, encoded);
                let (max, rms, code) = compare(&cpu.image, &gpu);
                records.push(json!({"case":name,"width":width,"height":height,"sampling":format!("{sampling:?}"),"maxLinearError":max,"rmsLinearError":rms,"maxRgba8Error":code,"allocatedBytes":stats.allocated_bytes}));
            }
        }
    }
    let plan = cases().pop().unwrap().1;
    let options = FrameRenderOptions {
        raster: RasterExtent {
            width: 13,
            height: 9,
        },
        sampling: Sampling::Bilinear,
        limits: RenderLimits::default(),
    };
    let calls = Cell::new(0);
    drop(
        pollster::block_on(compositor.encode_with_options(
            &plan,
            &mut resources,
            options,
            GpuLimits::default(),
            &|| {
                calls.set(calls.get() + 1);
                false
            },
        ))
        .unwrap(),
    );
    let points = calls.get();
    for point in 1..=points {
        calls.set(0);
        assert!(matches!(
            pollster::block_on(compositor.encode_with_options(
                &plan,
                &mut resources,
                options,
                GpuLimits::default(),
                &|| {
                    calls.set(calls.get() + 1);
                    calls.get() >= point
                }
            )),
            Err(RenderError::Cancelled)
        ));
    }
    let encoded = pollster::block_on(compositor.encode_with_options(
        &plan,
        &mut resources,
        options,
        GpuLimits::default(),
        &|| false,
    ))
    .unwrap();
    let recovered = readback(&device, &queue, encoded);
    let cpu = render_frame_with_options(&plan, &mut resources, options, &|| false).unwrap();
    compare(&cpu.image, &recovered);
    let report = json!({"format":"musteroffice.video-viewport-gpu/1","status":"passed","adapter":info.name,"backend":format!("{:?}",info.backend),"deviceType":format!("{:?}",info.device_type),"cases":records,"cancellationPoints":points,"scope":"Actual hardware CPU/GPU raster comparisons. Not display, zero-copy, real-time performance or product acceptance."});
    if let Some(path) = std::env::var_os("MO_VIDEO_VIEWPORT_GPU_REPORT") {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap()
            .write_all(serde_json::to_string_pretty(&report).unwrap().as_bytes())
            .unwrap();
    }
    println!("{report}");
}
