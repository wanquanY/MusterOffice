//! Explicit development host: immutable PPTX bytes -> native source compiler ->
//! actual Native glyph paths. This is not the product's page-rendering operation.
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_opc::{Package, PackageLimits};
use mo_pptx::source::{SourceLimits, SourceObjectRef, inspect_source};
use mo_presentation_compile::source_text::*;
use mo_text::{flow::OverflowPolicy, geometry::LineSpacing, manifest::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("expected owned PPTX path")?;
    let part = args.next().ok_or("expected source surface part")?;
    let native_id = args.next().ok_or("expected source object ID")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected arguments".into());
    }
    let bytes = std::fs::read(path)?;
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )?;
    let index = inspect_source(&package, SourceLimits::default(), &|| false)?;
    let source = prepare(
        &index,
        &index.source_sha256,
        &SourceObjectRef { part, native_id },
        SourceTextLimits::default(),
        &|| false,
    )?;
    let SourceTextPreparation::Prepared { text } = source else {
        return Err(format!("{source:?}").into());
    };
    let fixture: ManifestParagraphRequest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))?;
    let manifest = PreparedManifest::load(
        &fixture.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )?;
    let mut backend = NativeShaper::default();
    let mut results = vec![];
    for paragraph in 0..text.paragraphs().len() as u32 {
        let shape = text.shape_paragraph(paragraph, &manifest, &mut backend, &|| false);
        let shape = match shape {
            Ok(result) => serde_json::json!({"result":result}),
            Err(e) => serde_json::json!({"error":e.to_string()}),
        };
        for width in [200_000, 600_000] {
            let paths = text.paragraph_paths(
                paragraph,
                SourceGlyphFlow {
                    width: Emu::new(width),
                    spacing: LineSpacing::Natural,
                    overflow: OverflowPolicy::EmergencyGrapheme,
                    bounds_tolerance: Fixed::from_raw(1 << 26),
                },
                &manifest,
                &mut backend,
                &|| false,
            );
            let paths = match paths {
                Ok(result) => serde_json::json!({"result":result}),
                Err(e) => serde_json::json!({"error":e.to_string()}),
            };
            results.push(serde_json::json!({"paragraph":paragraph,"width":width,"shape":shape,"paths":paths}));
        }
    }
    println!(
        "{}",
        serde_json::json!({"source":text.source(),"paragraphs":text.paragraphs(),"verifiedFaces":manifest.verified_faces(),"results":results})
    );
    Ok(())
}
