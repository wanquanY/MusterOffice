//! Real pinned font corpus; the normal unit suite uses owned synthetic fonts.
use mo_harfbuzz_sys::NativeShaper;
use mo_text::{fallback::FontFragment, manifest::*};
use serde_json::json;

#[test]
#[ignore = "requires the explicitly downloaded fixtures/fonts/upstream.json corpus"]
fn manifest_fallback_shapes_mixed_scripts_and_preserves_primary_style() {
    let corpus =
        std::path::PathBuf::from(std::env::var_os("MO_FONT_CORPUS").expect("explicit font corpus"));
    let specs = [
        (
            "notosans/NotoSans[wdth,wght].ttf",
            "Noto Sans",
            "NotoSans-Regular",
            19,
            20,
            24,
            "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
        ),
        (
            "notosansarabic/NotoSansArabic[wdth,wght].ttf",
            "Noto Sans Arabic",
            "NotoSansArabic-Regular",
            10,
            11,
            15,
            "63111b5b2e074dd48cc67692e0a2726d86ee94c1c37fe8598257b7b4e87e869e",
        ),
        (
            "notosanssc/NotoSansSC[wght].ttf",
            "Noto Sans SC",
            "NotoSansSC-Thin",
            23,
            11,
            15,
            "a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da",
        ),
    ];
    let mut bundle = vec![];
    let mut fonts = vec![];
    let mut faces = vec![];
    let mut typefaces = vec![];
    for (i, (path, family, ps, family_record, style_record, ps_record, hash)) in
        specs.iter().enumerate()
    {
        let bytes = std::fs::read(corpus.join(path)).unwrap();
        fonts.push(json!({"expectedSha256":hash,"faceIndex":0,"offset":bundle.len().to_string(),"byteLength":bytes.len().to_string()}));
        bundle.extend(bytes);
        faces.push(json!({"font":i,"family":{"record":family_record,"expected":family},
            "subfamily":{"record":style_record,"expected":"Regular"},"postscript":{"record":ps_record,"expected":ps}}));
        typefaces.push(json!({"typeface":family,"policy":{"kind":"exactFamily"},
            "regular":{"face":i,"variations":[{"tag":"wght","value1616":400*65536}]},
            "bold":{"face":i,"variations":[{"tag":"wght","value1616":700*65536}]},"italic":null,"boldItalic":null}));
    }
    typefaces[0]["fallbacks"] = json!(["Noto Sans Arabic", "Noto Sans SC"]);
    let manifest = json!({"profile":"explicit-font-resource-manifest-draft-v1","fonts":fonts,"faces":faces,"typefaces":typefaces});
    for style in ["regular", "bold"] {
        for (text, required) in [
            ("Latin office", vec![0]),
            ("مرحبا", vec![1]),
            ("中文", vec![2]),
            ("Latin 中文 مرحبا", vec![0, 1, 2]),
        ] {
            let mut q: ManifestParagraphRequest = serde_json::from_value(json!({
                "manifest":manifest,"text":text,"direction":"leftToRight",
                "spans":[{"end":text.chars().count(),"style":0}],
                "styles":[{"typeface":"Noto Sans","fontStyle":style,"language":"und",
                    "features":[],"suppressDottedCircle":false,"maxGlyphs":1024}]
            }))
            .unwrap();
            let shaped = shape_paragraph(
                &q,
                &bundle,
                &mut NativeShaper::default(),
                ManifestLimits::default(),
                &|| false,
            )
            .unwrap();
            let mut used = std::collections::BTreeSet::new();
            for fragment in shaped
                .shaping
                .fallback
                .items
                .iter()
                .flat_map(|i| &i.fragments)
            {
                let FontFragment::Selected { font, shaped, .. } = fragment else {
                    panic!("unresolved text: {text}")
                };
                used.insert(*font);
                assert!(
                    shaped
                        .runs
                        .iter()
                        .flat_map(|r| &r.glyphs)
                        .all(|g| g.glyph_id != 0)
                );
            }
            assert!(
                required.iter().all(|font| used.contains(font)),
                "{text}: {used:?}"
            );
            if required.contains(&1) || required.contains(&2) {
                q.manifest.typefaces[0].fallbacks.clear();
                let without = shape_paragraph(
                    &q,
                    &bundle,
                    &mut NativeShaper::default(),
                    ManifestLimits::default(),
                    &|| false,
                )
                .unwrap();
                assert!(
                    without
                        .shaping
                        .fallback
                        .items
                        .iter()
                        .flat_map(|i| &i.fragments)
                        .any(|f| matches!(f, FontFragment::Unresolved { .. }))
                );
            }
        }
    }
}
