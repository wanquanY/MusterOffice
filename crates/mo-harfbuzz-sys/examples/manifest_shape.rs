//! Development-only isolated real-component probe. Uses owned fixture bytes;
//! does not add an in-process backend to any production application.
use mo_harfbuzz_sys::NativeShaper;
use mo_text::manifest::{ManifestLimits, ManifestParagraphRequest, shape_paragraph};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request: ManifestParagraphRequest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))?;
    let font = include_bytes!("../../../fixtures/fonts/owned.ttf");
    let result = shape_paragraph(
        &request,
        font,
        &mut NativeShaper::default(),
        ManifestLimits::default(),
        &|| false,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
