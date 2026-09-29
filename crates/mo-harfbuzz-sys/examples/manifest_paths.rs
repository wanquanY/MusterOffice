//! Owned Native resource/layout probe; no application backend embedding.
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_text::{
    cascade::FontCandidate,
    flow::{OverflowPolicy, ParagraphLayoutRequest},
    geometry::{GeometryStyle, LineSpacing},
    itemize::StyleSpan,
    manifest::{
        ManifestLayoutInput, ManifestLimits, ManifestParagraphInput, ManifestParagraphRequest,
        PreparedManifest,
    },
    paragraph::{ParagraphShapeRequest, ParagraphTextStyle},
    scene::{ParagraphPathsRequest, paragraph_paths},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture: ManifestParagraphRequest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))?;
    let bytes = include_bytes!("../../../fixtures/fonts/owned.ttf");
    let manifest =
        PreparedManifest::load(&fixture.manifest, bytes, ManifestLimits::default(), &|| {
            false
        })?;
    let mut backend = NativeShaper::default();
    let mut out = vec![];
    for text in [
        "",
        "A",
        "A A",
        "AA AA AAA",
        "A\nA",
        "A\r\n",
        "Aα A",
        "A\tA",
        "A\u{ad}A",
        "A\u{200d}A",
    ] {
        for width in [100_000, 200_000] {
            let request = ParagraphPathsRequest {
                layout: ParagraphLayoutRequest {
                    paragraph: ParagraphShapeRequest {
                        text: text.into(),
                        direction: fixture.direction,
                        fonts: fixture.manifest.fonts.clone(),
                        spans: if text.is_empty() {
                            vec![]
                        } else {
                            vec![StyleSpan {
                                end: text.chars().count() as u32,
                                style: 0,
                            }]
                        },
                        styles: vec![ParagraphTextStyle {
                            language: "en".into(),
                            features: vec![],
                            candidates: vec![FontCandidate {
                                font: 0,
                                variations: vec![],
                            }],
                            suppress_dotted_circle: false,
                            max_glyphs: 64,
                        }],
                    },
                    styles: vec![GeometryStyle {
                        cluster_spacing: mo_geometry::Fixed::ZERO,
                        font_size: Emu::new(100_000),
                        baseline_shift: Emu::ZERO.into(),
                    }],
                    strut_style: 0,
                    spacing: LineSpacing::Natural,
                    width: Emu::new(width),
                    overflow: OverflowPolicy::EmergencyGrapheme,
                    hanging_punctuation: mo_text::flow::HangingPunctuation::None,
                    wrapping: mo_text::flow::LineWrapping::Wrap,
                },
                bounds_tolerance: Fixed::from_raw(1 << 26),
            };
            let result = paragraph_paths(&request, bytes, &mut NativeShaper::default(), &|| false);
            let input = ManifestLayoutInput {
                paragraph: ManifestParagraphInput {
                    text,
                    direction: fixture.direction,
                    spans: &request.layout.paragraph.spans,
                    styles: &fixture.styles,
                },
                styles: &request.layout.styles,
                strut_style: request.layout.strut_style,
                width: request.layout.width,
                spacing: request.layout.spacing.clone(),
                overflow: request.layout.overflow,
            };
            let prepared =
                manifest.paragraph_paths(input, request.bounds_tolerance, &mut backend, &|| false);
            let legacy = match result {
                Ok(r) => serde_json::json!({"result":r}),
                Err(e) => serde_json::json!({"error":e.to_string()}),
            };
            let prepared = match prepared {
                Ok(r) => serde_json::json!({"result":r}),
                Err(e) => serde_json::json!({"error":e.to_string()}),
            };
            out.push(serde_json::json!({"request":request,"legacy":legacy,"prepared":prepared}));
        }
    }
    println!(
        "{}",
        serde_json::json!({"manifestVerifiedFaces":manifest.verified_faces(),"operations":out})
    );
    Ok(())
}
