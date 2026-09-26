//! One bounded document job per process. Package/index/fonts live for the batch;
//! page pixels are streamed and released before the next page is prepared.
use mo_kernel_api::*;
use std::io::{self, Read, Write};

pub fn run(author: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut header = [0; 12];
    input.read_exact(&mut header)?;
    let lengths: Vec<_> = header
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
        .collect();
    if lengths[0] > MAX_REQUEST_BYTES
        || lengths[1] > MAX_INLINE_RESOURCE_BYTES
        || lengths[2] > MAX_INLINE_FONT_BYTES
    {
        return Err("document preview input budget".into());
    }
    let mut json = vec![0; lengths[0]];
    let mut source = vec![0; lengths[1]];
    let mut fonts = vec![0; lengths[2]];
    input.read_exact(&mut json)?;
    input.read_exact(&mut source)?;
    input.read_exact(&mut fonts)?;
    let mut extra = [0; 1];
    if input.read(&mut extra)? != 0 {
        return Err("trailing document preview input".into());
    }
    if author {
        let request: AuthorResourceDocumentRequest =
            mo_common::from_json_str(std::str::from_utf8(&json)?)?;
        return render_author_resource_document(
            &request,
            &source,
            &fonts,
            PptxResourcePageBackends {
                decoder: &mut mo_skia_sys::NativeRaster,
                text: Some(&mut mo_harfbuzz_sys::NativeShaper::default()),
                raster: &mut mo_skia_sys::NativeRaster,
            },
            &|| false,
            &mut |response, pixels| write_frame(&mut output, response, pixels),
        );
    }
    let request: PptxResourceDocumentRequest =
        mo_common::from_json_str(std::str::from_utf8(&json)?)?;
    if request.pages.is_empty() || request.pages.len() > 256 {
        return Err("document preview page budget".into());
    }
    let prepared = match prepare_pptx_resource_document_inputs(
        &request.pages[0].page.expected_source_sha256,
        request.fonts.as_ref(),
        source.as_slice(),
        source.len() as u64,
        &fonts,
        &|| false,
    ) {
        Ok(prepared) => prepared,
        Err(error) => {
            write_frame(
                &mut output,
                PptxResourcePageRasterResponse::Error {
                    error: Box::new(error),
                },
                &[],
            )?;
            return Ok(());
        }
    };
    let mut text = mo_harfbuzz_sys::NativeShaper::default();
    let mut raster = mo_skia_sys::NativeRaster;
    let mut decoder = mo_skia_sys::NativeRaster;
    for page in &request.pages {
        let (metadata, pixels) = render_pptx_resource_document_page(
            &prepared,
            &page.page,
            page.image_source,
            page.sampling,
            PptxResourcePageBackends {
                decoder: &mut decoder,
                text: Some(&mut text),
                raster: &mut raster,
            },
            &|| false,
        );
        let failed = matches!(metadata, PptxResourcePageRasterResponse::Error { .. });
        write_frame(&mut output, metadata, &pixels)?;
        if failed || raster.is_invalid() || text.is_invalid() {
            break;
        }
    }
    Ok(())
}
fn write_frame(
    output: &mut impl Write,
    response: PptxResourcePageRasterResponse,
    pixels: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_vec(&response)?;
    output.write_all(&u32::try_from(json.len())?.to_le_bytes())?;
    output.write_all(&u32::try_from(pixels.len())?.to_le_bytes())?;
    output.write_all(&json)?;
    output.write_all(pixels)?;
    output.flush()?;
    Ok(())
}
