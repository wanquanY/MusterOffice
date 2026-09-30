mod image_decode;
mod playback;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compute_template(request: &str) -> String {
    mo_presentation_template::compute_template_json(request, Default::default(), &|| false)
}

/// Create, atomically edit or instantiate a template with the same computation
/// receipt as native callers. Import/export use the separate binary channels.
#[wasm_bindgen]
pub fn compute_document(invocation: &str) -> Result<String, String> {
    let result = mo_presentation_operations::decode_invocation(invocation)
        .and_then(|input| mo_presentation_operations::compute_inline(input, &|| false));
    match result {
        Ok(receipt) => Ok(serde_json::to_string(&receipt).expect("typed computation receipt")),
        Err(error) => Err(serde_json::to_string(&error).expect("typed computation failure")),
    }
}
mod raster;
mod text;

/// Pure calculation contracts; no permissions, jobs or storage capabilities.
#[wasm_bindgen]
pub fn computation_schema_json(id_json: &str) -> Result<String, String> {
    mo_presentation_operations::computation_schema_json(id_json).map_err(|error| error.to_string())
}

/// Shared receipt/byte binding checks. This creates no browser job owner and
/// does not authorize or commit an Artifact in the product.
#[wasm_bindgen]
pub fn inspect_delivery(request: &str, contents: &[u8]) -> String {
    mo_kernel_api::inspect_delivery_json(request, contents)
}

#[wasm_bindgen]
pub fn prepare_delivery_playback(request: &str, contents: &[u8]) -> String {
    mo_kernel_api::prepare_delivery_playback_json(request, contents)
}

#[wasm_bindgen]
pub fn inspect_pptx_charts(request: &str, source: &[u8]) -> String {
    mo_kernel_api::inspect_pptx_charts_json(request, source)
}

#[wasm_bindgen]
pub fn layout_chart_sectors(request: &str) -> String {
    mo_kernel_api::layout_chart_sectors_json(request, &|| false)
}

#[wasm_bindgen]
pub fn compile_chart_geometry(request: &str) -> String {
    mo_kernel_api::compile_chart_geometry_json(request, &|| false)
}

#[wasm_bindgen]
pub fn inspect_pptx_images(request: &str, source: &[u8]) -> String {
    mo_kernel_api::inspect_pptx_images_json(request, source)
}
#[wasm_bindgen]
pub struct ExtractedImages {
    metadata: String,
    bytes: Vec<u8>,
}
#[wasm_bindgen]
impl ExtractedImages {
    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> String {
        self.metadata.clone()
    }
    pub fn take_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
#[wasm_bindgen]
pub fn extract_pptx_images(request: &str, source: &[u8]) -> ExtractedImages {
    let (metadata, bytes) = mo_kernel_api::extract_pptx_images_json(request, source);
    ExtractedImages { metadata, bytes }
}

#[wasm_bindgen]
pub fn compile_pptx_page(request: &str, source: &[u8]) -> String {
    mo_kernel_api::compile_pptx_page_json(request, source)
}

#[wasm_bindgen]
pub fn compile_page(request: &str) -> String {
    mo_kernel_api::compile_page_json(request, &|| false)
}

#[wasm_bindgen]
pub fn page_placements(request: &str) -> String {
    mo_kernel_api::page_placements_json(request, &|| false)
}

/// Shared pure computation entry. Host owns workers, cancellation and publication.
#[wasm_bindgen]
pub fn dispatch_json(input: &str) -> String {
    mo_kernel_api::dispatch_json(input)
}

/// Development binary entry. Run in a host Worker; no path or implicit network access.
#[wasm_bindgen]
pub fn inspect_package(input: &[u8]) -> String {
    mo_kernel_api::inspect_package_json(input)
}

/// Native editable PPTX development export. Binary resources use checked ranges.
#[wasm_bindgen]
pub fn export_pptx(request: &str, resource_bytes: &[u8]) -> Result<Vec<u8>, String> {
    mo_kernel_api::export_pptx_json(request, resource_bytes)
}

#[wasm_bindgen]
pub fn inspect_pptx(input: &[u8]) -> String {
    mo_kernel_api::inspect_pptx_json(input)
}

#[wasm_bindgen]
pub fn edit_pptx_text(request: &str, source: &[u8]) -> Result<Vec<u8>, String> {
    mo_kernel_api::edit_pptx_text_json(request, source)
}

#[wasm_bindgen]
pub fn edit_pptx_transforms(request: &str, source: &[u8]) -> Result<Vec<u8>, String> {
    mo_kernel_api::edit_pptx_transforms_json(request, source)
}

#[wasm_bindgen]
pub fn resolve_pptx_colors(request: &str, source: &[u8]) -> String {
    mo_kernel_api::resolve_pptx_colors_json(request, source)
}

#[wasm_bindgen]
pub fn inspect_font(request: &str, bytes: &[u8]) -> String {
    mo_kernel_api::inspect_font_json(request, bytes)
}

#[wasm_bindgen]
pub fn analyze_text(request: &str) -> String {
    mo_kernel_api::analyze_text_json(request)
}
#[wasm_bindgen]
pub fn analyze_bidi(request: &str) -> String {
    mo_kernel_api::analyze_bidi_json(request)
}

#[wasm_bindgen]
pub fn analyze_line_breaks(request: &str) -> String {
    mo_kernel_api::analyze_line_breaks_json(request)
}

#[wasm_bindgen]
pub fn resolve_pptx_fill_colors(request: &str, source: &[u8]) -> String {
    mo_kernel_api::resolve_pptx_fill_colors_json(request, source)
}

#[wasm_bindgen]
pub fn resolve_pptx_table_borders(request: &str, source: &[u8]) -> String {
    mo_kernel_api::resolve_pptx_table_borders_json(request, source)
}

#[wasm_bindgen]
pub fn resolve_pptx_fills(request: &str, source: &[u8]) -> String {
    mo_kernel_api::resolve_pptx_fills_json(request, source)
}

#[wasm_bindgen]
pub fn resolve_pptx_lines(request: &str, source: &[u8]) -> String {
    mo_kernel_api::resolve_pptx_lines_json(request, source)
}

#[wasm_bindgen]
pub fn resolve_pptx_text_bodies(request: &str, source: &[u8]) -> String {
    mo_kernel_api::resolve_pptx_text_bodies_json(request, source)
}

#[wasm_bindgen]
pub fn resolve_pptx_line_colors(request: &str, source: &[u8]) -> String {
    mo_kernel_api::resolve_pptx_line_colors_json(request, source)
}

#[wasm_bindgen]
pub fn evaluate_pptx_geometry(request: &str, source: &[u8]) -> String {
    mo_kernel_api::evaluate_pptx_geometry_json(request, source)
}

#[wasm_bindgen]
pub fn compile_pptx_paths(request: &str, source: &[u8]) -> String {
    mo_kernel_api::compile_pptx_paths_json(request, source)
}
#[wasm_bindgen]
pub fn place_pptx_objects(request: &str, source: &[u8]) -> String {
    mo_kernel_api::place_pptx_objects_json(request, source)
}

#[wasm_bindgen]
pub fn layout_pptx_radial(request: &str, source: &[u8]) -> String {
    mo_kernel_api::layout_pptx_radial_json(request, source)
}

#[wasm_bindgen]
pub fn evaluate_timeline(request: &str) -> String {
    mo_kernel_api::evaluate_timeline_json(request, &|| false)
}
#[wasm_bindgen]
pub fn inspect_pptx_timing(request: &str, source: &[u8]) -> String {
    mo_kernel_api::inspect_pptx_timing_json(request, source)
}

#[wasm_bindgen]
pub fn compile_playback_page(request: &str) -> String {
    mo_kernel_api::compile_playback_page_json(request, &|| false)
}

#[wasm_bindgen]
pub fn import_pptx_document(request: &str, source: &[u8]) -> String {
    mo_kernel_api::import_pptx_document_json(request, source)
}
