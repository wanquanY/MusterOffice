//! Independent pixel masks and retained playback for native text overflow.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_playback.rs"]
mod support;
use mo_common::RationalTime;
use mo_harfbuzz_sys::NativeShaper;
use mo_opc::Package;
use mo_presentation_compile::{
    source_playback::SourcePlaybackPlan,
    source_resource_page::TextPageContext,
    source_text_page::{self, SourceTextPagePlan},
};
use mo_skia_sys::NativeRaster;
use mo_text::manifest::{FontManifest, PreparedManifest};
const FONT_BYTES: &[u8] = include_bytes!("../../../fixtures/fonts/owned-decorations.ttf");
fn fonts() -> FontManifest {
    serde_json::from_str(include_str!(
        "../../../fixtures/fonts/decoration-manifest.json"
    ))
    .unwrap()
}
use support::*;

fn source(mode: &str, angle: i32, flip: bool, scale: [i64; 2], decorated: bool) -> Vec<u8> {
    let run = colored("AAAA", "123456");
    let run = if decorated {
        run.replace("<a:rPr>", "<a:rPr u=\"sng\" strike=\"sngStrike\">")
    } else {
        run
    };
    let runs = std::iter::repeat_n(run, 3)
        .collect::<Vec<_>>()
        .join("<a:br/>");
    let attrs = match mode {
        "horizontal" => "horzOverflow=\"clip\"",
        "vertical" => "vertOverflow=\"clip\"",
        "both" => "horzOverflow=\"clip\" vertOverflow=\"clip\"",
        "none" => "",
        _ => panic!(),
    };
    let shape = shape(
        42,
        400_000,
        400_000,
        &format!("rot=\"{angle}\" flipH=\"{}\"", u8::from(flip)),
        &runs,
    )
    .replace(
        "cx=\"1000000\" cy=\"600000\"",
        "cx=\"240000\" cy=\"240000\"",
    )
    .replace("<a:bodyPr ", &format!("<a:bodyPr {attrs} "))
    .replace(
        "<a:solidFill><a:srgbClr val=\"F4EADC\"/></a:solidFill>",
        "<a:noFill/>",
    );
    let shape = if scale == [100, 100] {
        shape
    } else {
        group(
            90,
            &transform(
                "",
                [0, 0, 16_000 * scale[0], 12_000 * scale[1]],
                [0, 0, 1_600_000, 1_200_000],
            ),
            "<a:noFill/>",
            &shape,
        )
    };
    fixture(&shape)
}
fn render(bytes: &[u8]) -> (SourceTextPagePlan, Vec<u8>) {
    let index = read(bytes);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let plan = source_text_page::compile(
        &index,
        &request(&index),
        &manifest,
        &mut NativeShaper::default(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let image = source_text_page::render(
        &index,
        &request(&index),
        &manifest,
        &mut NativeShaper::default(),
        &mut NativeRaster,
        Default::default(),
        &|| false,
    )
    .unwrap();
    image
        .info
        .text_capacity
        .as_ref()
        .unwrap()
        .validate(1, &|| false)
        .unwrap();
    (plan, image.pixels)
}
fn evidence(name: &str, bytes: &[u8], plan: &SourceTextPagePlan, pixels: &[u8]) {
    if let Some(dir) = std::env::var_os("MO_TEXT_CLIP_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.pptx")), bytes).unwrap();
        std::fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(plan).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join(format!("{name}.rgba")), pixels).unwrap();
    }
}

#[test]
fn native_axis_clipping_matches_independent_pixel_masks_and_preserves_ink() {
    for decorated in [false, true] {
        for (angle, flip, scale) in [
            (0, false, [100, 100]),
            (2_700_000, false, [100, 100]),
            (5_400_000, true, [100, 100]),
            (2_700_000, true, [125, 75]),
        ] {
            let reference_bytes = source("none", angle, flip, scale, decorated);
            let (reference, original) = render(&reference_bytes);
            evidence(
                &format!("none-{angle}-{flip}-{}-{}-{decorated}", scale[0], scale[1]),
                &reference_bytes,
                &reference,
                &original,
            );
            for mode in ["horizontal", "vertical", "both"] {
                let bytes = source(mode, angle, flip, scale, decorated);
                let (plan, pixels) = render(&bytes);
                evidence(
                    &format!(
                        "{mode}-{angle}-{flip}-{}-{}-{decorated}",
                        scale[0], scale[1]
                    ),
                    &bytes,
                    &plan,
                    &pixels,
                );
                let a = &reference.page.raster.scene;
                let b = &plan.page.raster.scene;
                assert_eq!(a.instances.len(), b.instances.len());
                for (a_draw, b_draw) in a.instances.iter().zip(&b.instances) {
                    assert_eq!(
                        serde_json::to_value(&a.paths[a_draw.path as usize]).unwrap(),
                        serde_json::to_value(&b.paths[b_draw.path as usize]).unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(a_draw.transform.map(|i| &a.transforms[i as usize]))
                            .unwrap(),
                        serde_json::to_value(b_draw.transform.map(|i| &b.transforms[i as usize]))
                            .unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(&a_draw.brush).unwrap(),
                        serde_json::to_value(&b_draw.brush).unwrap()
                    );
                }
                let frame = &plan.texts[0].frame;
                assert_eq!(frame.glyphs.len(), reference.texts[0].frame.glyphs.len());
                assert_eq!(frame.bounds, reference.texts[0].frame.bounds);
                let capacity =
                    mo_presentation_compile::source_frame::capacity::measure(frame, &|| false)
                        .unwrap();
                assert!(capacity.vertical_excess > mo_geometry::Fixed::ZERO);
                assert_eq!(capacity.ink_bounds, frame.bounds);
                assert_eq!(
                    plan.texts[0].decorations.len(),
                    reference.texts[0].decorations.len()
                );
                if decorated {
                    assert!(!plan.texts[0].decorations.is_empty());
                }
                let (sin, cos) = (f64::from(angle) / 60_000.0).to_radians().sin_cos();
                let mut removed = 0;
                let mut preserved = 0;
                let mut inactive_ink = 0;
                for (index, (got, before)) in pixels
                    .chunks_exact(4)
                    .zip(original.chunks_exact(4))
                    .enumerate()
                {
                    // Independently invert the XML transform, in EMUs. The
                    // 16,000-local-EMU guard excludes antialiased active edges.
                    let dx = (index % 400) as f64 * 4000.0 + 2000.0;
                    let dy = (index / 400) as f64 * 4000.0 + 2000.0;
                    let x = dx - 520_000.0 * scale[0] as f64 / 100.0;
                    let y = dy - 520_000.0 * scale[1] as f64 / 100.0;
                    // DrawingML group sizing swaps axes at 45/135/225/315
                    // degree sectors. It is not a generic parent S*R chain.
                    // See the independent WPS-backed group placement corpus.
                    let a = angle.rem_euclid(21_600_000);
                    let swapped = (2_700_000..8_100_000).contains(&a)
                        || (13_500_000..18_900_000).contains(&a);
                    let sx = scale[usize::from(swapped)] as f64 / 100.0;
                    let sy = scale[usize::from(!swapped)] as f64 / 100.0;
                    let local = [
                        (cos * x + sin * y) / sx * if flip { -1.0 } else { 1.0 } + 120_000.0,
                        (-sin * x + cos * y) / sy + 120_000.0,
                    ];
                    let active = [mode != "vertical", mode != "horizontal"];
                    let outside =
                        (0..2).any(|a| active[a] && (local[a] < -16_000.0 || local[a] > 256_000.0));
                    let inside =
                        (0..2).all(|a| !active[a] || (16_000.0..224_000.0).contains(&local[a]));
                    if outside {
                        assert_eq!(
                            got, [255; 4],
                            "{mode}/{angle}/{flip}/{scale:?} at {index}, local {local:?}"
                        );
                        removed += usize::from(before != [255; 4]);
                    } else if inside {
                        // The pinned analytic rasterizer clips edges before
                        // scan conversion. Edge coverage can be requantized;
                        // unchanged path coordinates are checked separately.
                        // Interior colors must stay within the unmasked 3x3
                        // coverage envelope (all solid/empty interiors exact).
                        for channel in 0..4 {
                            let mut min = 255;
                            let mut max = 0;
                            for dy in -1isize..=1 {
                                for dx in -1isize..=1 {
                                    let x = (index % 400) as isize + dx;
                                    let y = (index / 400) as isize + dy;
                                    if (0..400).contains(&x) && (0..300).contains(&y) {
                                        let c =
                                            original[(y as usize * 400 + x as usize) * 4 + channel];
                                        min = min.min(c);
                                        max = max.max(c);
                                    }
                                }
                            }
                            // CPU AA clip/draw composition has one-LSB
                            // rounding; it must not move ink off this edge.
                            assert!(
                                (min.saturating_sub(1)..=max.saturating_add(1))
                                    .contains(&got[channel]),
                                "interior ink escaped its pixel neighbourhood: {mode}/{angle}/{flip}/{scale:?}, {index}: {got:?} vs {before:?}, local {local:?}"
                            );
                        }
                        preserved += usize::from(got != [255; 4]);
                        if got != [255; 4]
                            && (0..2).any(|a| {
                                !active[a] && (local[a] < -16_000.0 || local[a] > 256_000.0)
                            })
                        {
                            inactive_ink += 1;
                        }
                    }
                }
                assert!(
                    removed > 10 && preserved > 10,
                    "mask has no evidence: {mode}/{angle} {removed}/{preserved}"
                );
                if mode != "both" {
                    assert!(inactive_ink > 10, "inactive axis lost: {mode}/{angle}");
                }
                let clip = plan.page.raster.scene.clips.last().unwrap();
                assert_eq!(clip.parent, None);
                evidence(
                    &format!(
                        "{mode}-{angle}-{flip}-{}-{}-{decorated}",
                        scale[0], scale[1]
                    ),
                    &bytes,
                    &plan,
                    &pixels,
                );
            }
        }
    }
}

struct Never;
impl mo_image::ImageDecoder for Never {
    fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        panic!("text fixture has no images")
    }
    fn invalidate(&mut self) {
        panic!("unexpected invalidation")
    }
}

#[test]
fn cell_style_horizontal_clip_keeps_the_page_boundary_parent() {
    let bytes = rewrite(
        &source("horizontal", 0, false, [100, 100], true),
        "/ppt/presentation.xml",
        |s| {
            s.replace(
                "cx=\"1600000\" cy=\"1200000\"",
                "cx=\"1601000\" cy=\"1201000\"",
            )
        },
    );
    let index = read(&bytes);
    let mut q = request(&index);
    q.viewport.width = 401;
    q.viewport.height = 301;
    q.viewport.background = [0, 0, 0, 255];
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let plan = source_text_page::compile(
        &index,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(plan.page.raster.scene.clips.len(), 2);
    assert_eq!(plan.page.raster.scene.clips[1].parent, Some(0));
    for source in &plan.text_sources {
        assert_eq!(
            plan.page.raster.scene.instances[source.instance as usize].clip,
            Some(1)
        );
    }
    let image = source_text_page::render(
        &index,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        &mut NativeRaster,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert!(
        image
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel == [18, 52, 86, 255])
    );
    for (i, pixel) in image.pixels.chunks_exact(4).enumerate() {
        if i / 401 >= 300 || i % 401 == 400 {
            assert!(
                pixel[..3].iter().all(|c| *c <= 64),
                "ink escaped fractional page clip: {i} {pixel:?}"
            );
        }
    }
}

#[test]
fn retained_clips_are_resampled_with_rotation_and_nonuniform_animation_scale() {
    for mode in ["horizontal", "vertical", "both"] {
        let base = source(mode, 0, true, [125, 75], true);
        for (kind, bytes) in [
            ("rotation", animated(&base, 42, 0, 21_600_000, "freeze")),
            (
                "scale",
                scaled(&base, 42, [100_000, 100_000], [20_000, 170_000], "freeze"),
            ),
            (
                "zero-x",
                scaled(&base, 42, [0, 100_000], [100_000, 100_000], "freeze"),
            ),
            (
                "zero-y",
                scaled(&base, 42, [100_000, 0], [100_000, 100_000], "freeze"),
            ),
            (
                "zero-both",
                scaled(&base, 42, [0, 0], [100_000, 100_000], "freeze"),
            ),
        ] {
            let index = read(&bytes);
            let package = Package::open(
                bytes.as_slice(),
                bytes.len() as u64,
                Default::default(),
                &|| false,
            )
            .unwrap();
            let make_plan = || {
                SourcePlaybackPlan::new(&package,&index,request(&index),serde_json::from_value(serde_json::json!({"session":"clip-test","generation":"1","revision":index.source_sha256})).unwrap(),Default::default(),Default::default(),&||false).unwrap()
            };
            let fonts = fonts();
            let manifest =
                PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
            let mut retained = make_plan()
                .retain(
                    &package,
                    index.clone(),
                    &mut Never,
                    Some(TextPageContext {
                        manifest: &manifest,
                        backend: &mut NativeShaper::default(),
                    }),
                    options(),
                    &|| false,
                )
                .unwrap();
            let mut direct = make_plan();
            let mut previous = None;
            for (ordinal, at) in [0, 250, 500, 750, 1000, 250].into_iter().enumerate() {
                let at = RationalTime::new(at, 1000).unwrap();
                let (state, result) = retained
                    .render(at, None, &mut NativeRaster, &|| false)
                    .unwrap();
                if ordinal == 0 && kind.starts_with("zero-") {
                    assert!(result.pixels.chunks_exact(4).all(|pixel| pixel == [255; 4]));
                }
                let sample = direct.sample(at, None, &|| false).unwrap();
                let expected = sample
                    .prepare(
                        &package,
                        &index,
                        &mut Never,
                        Some(TextPageContext {
                            manifest: &manifest,
                            backend: &mut NativeShaper::default(),
                        }),
                        options(),
                        &|| false,
                    )
                    .unwrap()
                    .render(&mut NativeRaster, &|| false)
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(&state).unwrap(),
                    serde_json::to_value(sample.frame()).unwrap()
                );
                assert_eq!(result.pixels, expected.pixels, "{mode}/{kind} at {at:?}");
                assert_eq!(result.info.text_work.component_calls, 0);
                assert!(expected.info.text_work.component_calls > 0);
                assert_eq!(
                    serde_json::to_value(&result.info.text_capacity).unwrap(),
                    serde_json::to_value(&expected.info.text_capacity).unwrap()
                );
                if let Some(old) = previous {
                    assert_ne!(
                        result.pixels, old,
                        "sample did not transform the ink: {mode}/{kind}"
                    );
                }
                if let Some(dir) = std::env::var_os("MO_TEXT_CLIP_EVIDENCE_DIR") {
                    let dir = std::path::PathBuf::from(dir).join("playback");
                    std::fs::create_dir_all(&dir).unwrap();
                    let name = format!("{mode}-{kind}");
                    if ordinal == 0 {
                        std::fs::write(dir.join(format!("{name}.pptx")), &bytes).unwrap();
                    }
                    std::fs::write(dir.join(format!("{name}-{ordinal}.rgba")), &result.pixels)
                        .unwrap();
                    std::fs::write(
                        dir.join(format!("{name}-{ordinal}.state.json")),
                        serde_json::to_vec_pretty(&state).unwrap(),
                    )
                    .unwrap();
                }
                previous = Some(result.pixels);
            }
        }
    }
}
