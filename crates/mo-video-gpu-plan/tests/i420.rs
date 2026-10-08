use mo_video_gpu_plan::I420Parameters;
use mo_video_render::{ToneMap, i420_chroma_weights};

#[test]
fn output_abi_is_bounded_and_uses_reference_filter() {
    assert_eq!(std::mem::size_of::<I420Parameters>(), 176);
    let words = I420Parameters::new(6, 4, false, None, [0, 0, 0])
        .unwrap()
        .words();
    assert_eq!(&words[0..4], &[6, 4, 0, 0]);
    assert_eq!(&words[40..44], &[8, 4, 32, 40]);
    let (horizontal, vertical) = i420_chroma_weights();
    for (i, v) in horizontal.into_iter().chain(vertical).enumerate() {
        assert_eq!(words[24 + i], (v as f32).to_bits());
    }
    for (w, h) in [(0, 2), (2, 0), (3, 2), (2, 3), (8194, 2)] {
        assert!(I420Parameters::new(w, h, false, None, [0, 0, 0]).is_err());
    }
    assert!(I420Parameters::new(2, 2, true, None, [0, 0, 0]).is_err());
    assert!(
        I420Parameters::new(
            2,
            2,
            true,
            Some(ToneMap::ReinhardLuminanceV1 {
                source_peak_nits: f64::NAN
            }),
            [0, 0, 0]
        )
        .is_err()
    );
    assert!(
        I420Parameters::new(
            8192,
            8192,
            true,
            Some(ToneMap::ReinhardLuminanceV1 {
                source_peak_nits: 1000.
            }),
            [255, 255, 255]
        )
        .is_ok()
    );
}
