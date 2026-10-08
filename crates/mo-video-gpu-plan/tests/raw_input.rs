use mo_video_gpu_plan::{BrowserColorSpace, BrowserFrameInput, BrowserHlgDisplay};
fn source(format: &str) -> BrowserFrameInput {
    BrowserFrameInput {
        width: 5,
        height: 3,
        format: format.into(),
        color_space: BrowserColorSpace {
            primaries: "bt709".into(),
            transfer: "bt709".into(),
            matrix: "bt709".into(),
            full_range: false,
        },
        chroma_location: Some("left".into()),
        hlg_display: None,
    }
}
#[test]
fn padded_odd_planes_and_uniform_copy_layout_are_one_plan() {
    let p = source("I420").compile().unwrap();
    assert_eq!(p.byte_length, 40);
    assert_eq!(
        p.planes
            .iter()
            .map(|v| (v.offset, v.stride))
            .collect::<Vec<_>>(),
        vec![(0, 8), (24, 4), (32, 4)]
    );
    assert_eq!(p.parameters.len(), 56);
    assert_eq!(&p.parameters[..8], &[5, 3, 1, 8, 2, 2, 1, 0]);
    let p = source("NV12").compile().unwrap();
    assert_eq!(p.byte_length, 40);
    assert_eq!(p.planes.len(), 2);
    assert_eq!(p.parameters[7], 256);
    let p = source("I420P10").compile().unwrap();
    assert_eq!(p.byte_length, 68);
    assert_eq!(p.parameters[3], 10);
    assert_eq!(p.parameters[6], 2);
}
#[test]
fn color_and_siting_are_required_before_allocating_pixels() {
    let mut p = source("I420");
    p.chroma_location = None;
    assert!(p.compile().is_err());
    let mut p = source("I444");
    p.chroma_location = None;
    assert!(p.compile().is_ok());
    let mut p = source("RGBA");
    assert!(p.clone().compile().is_err());
    p.color_space.matrix = "rgb".into();
    assert!(p.compile().is_ok());
    let mut p = source("I420");
    p.color_space.primaries = "unspecified".into();
    assert!(p.compile().is_err());
    let mut p = source("I420");
    p.width = 16385;
    assert!(p.compile().is_err());
    let mut p = source("I444P12");
    p.width = 16384;
    p.height = 16384;
    assert!(p.compile().is_err());
}
#[test]
fn hdr_interpretation_is_explicit_and_preserves_source_metadata() {
    let mut p = source("I420P10");
    p.color_space.transfer = "hlg".into();
    p.color_space.primaries = "bt2020".into();
    p.color_space.matrix = "bt2020-ncl".into();
    assert!(p.clone().compile().is_err());
    p.hlg_display = Some(BrowserHlgDisplay {
        peak_nits: 1000.,
        system_gamma: 1.2,
    });
    let plan = p.clone().compile().unwrap();
    assert!(plan.has_hdr);
    assert_eq!(f32::from_bits(plan.parameters[48]), 6.);
    assert_eq!(plan.source.color_space.transfer, "hlg");
    p.hlg_display.as_mut().unwrap().system_gamma = 0.;
    assert!(p.compile().is_err());
}
