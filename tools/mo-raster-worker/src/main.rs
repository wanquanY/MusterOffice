//! Isolated raster worker. Explicit bytes only; the parent controls publication.
use mo_harfbuzz_sys::NativeShaper;
use mo_skia_sys::NativeRaster;
mod editor_page;
mod playback;
mod preview_batch;
mod source_playback;
use std::io::{self, Read, Write};
type Execute = fn(&str, &mut dyn mo_raster::RasterBackend, &dyn Fn() -> bool) -> (String, Vec<u8>);
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--editor-page-session" {
        return editor_page::run();
    }
    if args.len() == 1 && args[0] == "--playback-session" {
        return playback::run();
    }
    if args.len() == 1 && args[0] == "--pptx-playback-session" {
        return source_playback::run();
    }
    if args.len() == 1
        && let Some(mode) = args[0].to_str()
        && [
            "--preview-document",
            "--preview-author-document",
            "--preview-retained-document",
        ]
        .contains(&mode)
    {
        return preview_batch::run(mode);
    }
    let execute: Option<Execute> = match args.as_slice() {
        [] => Some(mo_kernel_api::render_paths_json),
        [mode] if mode == "--scene" => Some(mo_kernel_api::render_scene_json),
        [mode] if mode == "--page" => Some(mo_kernel_api::render_page_json),
        [mode] if mode == "--playback-page" => Some(mo_kernel_api::render_playback_page_json),
        [mode]
            if mode == "--pptx-page"
                || mode == "--pptx-text-page"
                || mode == "--pptx-resource-page"
                || mode == "--pptx-playback-page"
                || mode == "--images"
                || mode == "--image-scene"
                || mode == "--decode-image" =>
        {
            None
        }
        _ => {
            return Err(
                "mo-raster-worker accepts only an optional --scene, --page, --playback-page, --playback-session, --pptx-playback-session, --pptx-page, --pptx-playback-page, --pptx-text-page, --pptx-resource-page, --images --image-scene or --decode-image mode, no paths".into(),
            );
        }
    };
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut backend = NativeRaster;
    let mut text = NativeShaper::default();
    let resource_page = args.first().is_some_and(|a| a == "--pptx-resource-page");
    let source_playback = args.first().is_some_and(|a| a == "--pptx-playback-page");
    let text_page = args.first().is_some_and(|a| a == "--pptx-text-page");
    let decode_image = args.first().is_some_and(|a| a == "--decode-image");
    let image_scene = args.first().is_some_and(|a| a == "--image-scene");
    let image_paths = image_scene || args.first().is_some_and(|a| a == "--images");
    loop {
        let mut header = [0; 4];
        if input.read(&mut header[..1])? == 0 {
            break;
        }
        input.read_exact(&mut header[1..])?;
        let count = u32::from_le_bytes(header) as usize;
        if count > mo_kernel_api::MAX_REQUEST_BYTES {
            return Err("raster worker request limit".into());
        }
        let source_count = if execute.is_none() {
            let mut header = [0; 4];
            input.read_exact(&mut header)?;
            let count = u32::from_le_bytes(header) as usize;
            let limit = if decode_image {
                mo_image::MAX_ENCODED_BYTES
            } else if image_paths {
                mo_raster::MAX_PIXEL_BYTES
            } else {
                mo_kernel_api::MAX_INLINE_RESOURCE_BYTES
            };
            if count > limit {
                return Err("raster resource bytes limit".into());
            }
            count
        } else {
            0
        };
        let font_count = if text_page || resource_page || source_playback {
            let mut header = [0; 4];
            input.read_exact(&mut header)?;
            let count = u32::from_le_bytes(header) as usize;
            if count > mo_kernel_api::MAX_INLINE_FONT_BYTES {
                return Err("text page font bytes limit".into());
            }
            count
        } else {
            0
        };
        let mut bytes = vec![0; count];
        input.read_exact(&mut bytes)?;
        let mut source = vec![0; source_count];
        input.read_exact(&mut source)?;
        let mut fonts = vec![0; font_count];
        input.read_exact(&mut fonts)?;
        let (metadata, pixels) = match execute {
            Some(execute) => execute(std::str::from_utf8(&bytes)?, &mut backend, &|| false),
            None if decode_image => mo_kernel_api::decode_image_json(
                std::str::from_utf8(&bytes)?,
                &source,
                &mut backend,
                &|| false,
            ),
            None if image_paths => {
                let execute = if image_scene {
                    mo_kernel_api::render_image_scene_json
                } else {
                    mo_kernel_api::render_image_paths_json
                };
                execute(std::str::from_utf8(&bytes)?, &source, &mut backend, &|| {
                    false
                })
            }
            None if resource_page || source_playback => {
                let render = if source_playback {
                    mo_kernel_api::render_pptx_playback_page_json
                } else {
                    mo_kernel_api::render_pptx_resource_page_json
                };
                render(
                    std::str::from_utf8(&bytes)?,
                    &source,
                    &fonts,
                    mo_kernel_api::PptxResourcePageBackends {
                        decoder: &mut NativeRaster,
                        text: Some(&mut text),
                        raster: &mut backend,
                    },
                    &|| false,
                )
            }
            None if text_page => mo_kernel_api::render_pptx_text_page_json(
                std::str::from_utf8(&bytes)?,
                &source,
                &fonts,
                &mut text,
                &mut backend,
                &|| false,
            ),
            None => mo_kernel_api::render_pptx_page_json(
                std::str::from_utf8(&bytes)?,
                &source,
                &mut backend,
                &|| false,
            ),
        };
        output.write_all(&u32::try_from(metadata.len())?.to_le_bytes())?;
        output.write_all(&u32::try_from(pixels.len())?.to_le_bytes())?;
        output.write_all(metadata.as_bytes())?;
        output.write_all(&pixels)?;
        output.flush()?;
        if backend.is_invalid() || text.is_invalid() {
            break;
        }
    }
    Ok(())
}
