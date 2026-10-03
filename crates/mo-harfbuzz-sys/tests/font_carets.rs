use mo_harfbuzz_sys::NativeShaper;
use mo_text::{backend::*, carets::*};
const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/owned-carets.ttf");
#[test]
fn resident_caret_queries_reuse_owned_font_bytes_and_exact_unshaped_positions() {
    let mut component = NativeShaper::default();
    let frame = [
        CARETS_BATCH_MAGIC,
        1,
        HARFBUZZ_VERSION,
        1,
        11,
        CARETS_MAGIC,
        1,
        0,
        0,
        4,
        3,
        64,
        0,
        3,
        5,
        6,
    ];
    let before = component.caret_batch(FONT, &frame).unwrap();
    assert_eq!(
        before,
        vec![
            0,
            1,
            16,
            CARETS_MAGIC,
            1,
            1000,
            64000,
            4,
            3,
            3,
            1,
            8000,
            5,
            1,
            15680,
            6,
            2,
            11072,
            27584
        ]
    );
    let mut caller = FONT.to_vec();
    let handle = component.register_font(&caller).unwrap();
    caller.fill(0);
    for _ in 0..3 {
        assert_eq!(component.caret_registered(handle, &frame).unwrap(), before);
    }
    component.unregister_font(handle).unwrap();
    assert!(component.caret_registered(handle, &frame).is_err());
    let request: FontCaretsRequest = serde_json::from_value(serde_json::json!({
        "expectedSha256":"29e12bbc2c00eff9d3b4164b2bf02b8289e7d4bea58be489ab6f423353f1b4a8",
        "faceIndex":0,"instances":[{"variations":[],"direction":"leftToRight","glyphIds":[3,5,6]}]
    }))
    .unwrap();
    let carets = query(&request, FONT, &mut component, &|| false).unwrap();
    assert_eq!(carets.instances[0].glyphs[1].positions, [15680]);
    assert_eq!(carets.instances[0].glyphs[2].positions, [11072, 27584]);
    // The natural advance is 600 design units. Neither nonuniform caret is a
    // simple 1/3 or 2/3 split; later layout must apply the shaped glyph origin.
    assert_ne!(carets.instances[0].glyphs[2].positions, [12800, 25600]);
}
