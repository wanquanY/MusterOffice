use mo_common::{Digest, from_json_str};
use mo_kernel_api::{PageRasterResponse, PathRasterResponse, SceneRasterResponse};
use sha2::{Digest as _, Sha256};
use std::{error::Error, ffi::OsStr, io::Read, process::Command, time::Duration};
pub(super) enum Mode<'a> {
    Paths,
    Scene,
    Page,
    PlaybackPage,
    SourcePage(&'a OsStr),
    SourceTextPage(&'a OsStr, &'a OsStr),
    SourceResourcePage(&'a OsStr, &'a OsStr),
    SourcePlaybackPage(&'a OsStr, &'a OsStr),
}
pub(super) fn render(
    request: &OsStr,
    output: &OsStr,
    mode: Mode<'_>,
) -> Result<String, Box<dyn Error>> {
    let request = super::artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
    let executable = std::env::current_exe()?.with_file_name(if cfg!(windows) {
        "mo-raster-worker.exe"
    } else {
        "mo-raster-worker"
    });
    let header = u32::try_from(request.len())?.to_le_bytes().to_vec();
    let mut command = Command::new(executable);
    let mut chunks = vec![header, request];
    match mode {
        Mode::SourcePage(source)
        | Mode::SourceTextPage(source, _)
        | Mode::SourcePlaybackPage(source, _)
        | Mode::SourceResourcePage(source, _) => {
            command.arg(if matches!(mode, Mode::SourcePlaybackPage(..)) {
                "--pptx-playback-page"
            } else if matches!(mode, Mode::SourceResourcePage(..)) {
                "--pptx-resource-page"
            } else if matches!(mode, Mode::SourceTextPage(..)) {
                "--pptx-text-page"
            } else {
                "--pptx-page"
            });
            let source =
                super::artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            chunks[0].extend_from_slice(&u32::try_from(source.len())?.to_le_bytes());
            chunks.push(source);
            if let Mode::SourceTextPage(_, fonts)
            | Mode::SourceResourcePage(_, fonts)
            | Mode::SourcePlaybackPage(_, fonts) = mode
            {
                let fonts =
                    super::artifact::read_limited(fonts, mo_kernel_api::MAX_INLINE_FONT_BYTES)?;
                chunks[0].extend_from_slice(&u32::try_from(fonts.len())?.to_le_bytes());
                chunks.push(fonts);
            }
        }
        Mode::Paths => {}
        Mode::Scene => {
            command.arg("--scene");
        }
        Mode::PlaybackPage => {
            command.arg("--playback-page");
        }
        Mode::Page => {
            command.arg("--page");
        }
    }
    let (metadata, pixels) =
        super::worker::exchange(command, chunks, Duration::from_secs(30), |output| {
            let mut header = [0; 8];
            output.read_exact(&mut header).map_err(|e| e.to_string())?;
            let meta = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
            let pixel = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
            if meta > 64 * 1024 * 1024 || pixel > mo_raster::MAX_PIXEL_BYTES {
                return Err("raster response byte limit".into());
            }
            let mut metadata = vec![0; meta];
            let mut pixels = vec![0; pixel];
            output
                .read_exact(&mut metadata)
                .map_err(|e| e.to_string())?;
            output.read_exact(&mut pixels).map_err(|e| e.to_string())?;
            Ok((
                String::from_utf8(metadata).map_err(|e| e.to_string())?,
                pixels,
            ))
        })?;
    let info = match mode {
        Mode::SourcePlaybackPage(..) => {
            match from_json_str::<mo_kernel_api::PptxPlaybackRasterResponse>(&metadata)? {
                mo_kernel_api::PptxPlaybackRasterResponse::Error { .. } => None,
                mo_kernel_api::PptxPlaybackRasterResponse::Rendered { info } => {
                    if info.profile != info.playback.profile()
                        || info.page.profile != "drawingml-resource-page-q32-v1-draft"
                        || !matches!(
                            info.page.page.page.profile,
                            mo_kernel_api::SourcePageProfile::StaticSolidDraftV1
                        )
                        || info.page.page.scene.profile != mo_kernel_api::SCENE_RASTER_PROFILE
                    {
                        return Err("source playback profile mismatch".into());
                    }
                    Some(info.page.page.scene.raster)
                }
            }
        }
        Mode::SourceResourcePage(..) => {
            match from_json_str::<mo_kernel_api::PptxResourcePageRasterResponse>(&metadata)? {
                mo_kernel_api::PptxResourcePageRasterResponse::Error { .. } => None,
                mo_kernel_api::PptxResourcePageRasterResponse::Rendered { info } => {
                    if info.profile != "drawingml-resource-page-q32-v1-draft"
                        || !matches!(
                            info.page.page.profile,
                            mo_kernel_api::SourcePageProfile::StaticSolidDraftV1
                        )
                        || info.page.scene.profile != mo_kernel_api::SCENE_RASTER_PROFILE
                    {
                        return Err("source resource page profile mismatch".into());
                    }
                    Some(info.page.scene.raster)
                }
            }
        }
        Mode::SourceTextPage(..) => {
            match from_json_str::<mo_kernel_api::PptxTextPageRasterResponse>(&metadata)? {
                mo_kernel_api::PptxTextPageRasterResponse::Error { .. } => None,
                mo_kernel_api::PptxTextPageRasterResponse::Rendered { info } => {
                    if info.profile != "drawingml-solid-text-page-q32-draft-v1"
                        || !matches!(
                            info.page.page.profile,
                            mo_kernel_api::SourcePageProfile::StaticSolidDraftV1
                        )
                        || info.page.scene.profile != mo_kernel_api::SCENE_RASTER_PROFILE
                    {
                        return Err("source text page profile mismatch".into());
                    }
                    Some(info.page.scene.raster)
                }
            }
        }
        Mode::SourcePage(_) => {
            match from_json_str::<mo_kernel_api::PptxPageRasterResponse>(&metadata)? {
                mo_kernel_api::PptxPageRasterResponse::Error { .. } => None,
                mo_kernel_api::PptxPageRasterResponse::Rendered { info } => {
                    if !matches!(
                        info.page.profile,
                        mo_kernel_api::SourcePageProfile::StaticSolidDraftV1
                    ) || info.scene.profile != mo_kernel_api::SCENE_RASTER_PROFILE
                    {
                        return Err("source page profile mismatch".into());
                    }
                    Some(info.scene.raster)
                }
            }
        }
        Mode::PlaybackPage => {
            match from_json_str::<mo_kernel_api::PlaybackRasterResponse>(&metadata)? {
                mo_kernel_api::PlaybackRasterResponse::Error { .. } => None,
                mo_kernel_api::PlaybackRasterResponse::Rendered { info } => {
                    if info.profile != mo_kernel_api::frame_profile(&info.frame)
                        || info.page.page.profile != mo_kernel_api::PAGE_PROFILE
                        || info.page.scene.profile != mo_kernel_api::SCENE_RASTER_PROFILE
                    {
                        return Err("playback page profile mismatch".into());
                    }
                    Some(info.page.scene.raster)
                }
            }
        }
        Mode::Page => match from_json_str::<PageRasterResponse>(&metadata)? {
            PageRasterResponse::Error { .. } => None,
            PageRasterResponse::Rendered { info } => {
                if info.page.profile != mo_kernel_api::PAGE_PROFILE
                    || info.scene.profile != mo_kernel_api::SCENE_RASTER_PROFILE
                {
                    return Err("page profile mismatch".into());
                }
                Some(info.scene.raster)
            }
        },
        Mode::Paths => match from_json_str::<PathRasterResponse>(&metadata)? {
            PathRasterResponse::Error { .. } => None,
            PathRasterResponse::Rendered { info } => Some(*info),
        },
        Mode::Scene => match from_json_str::<SceneRasterResponse>(&metadata)? {
            SceneRasterResponse::Error { .. } => None,
            SceneRasterResponse::Rendered { info } => {
                if info.profile != mo_kernel_api::SCENE_RASTER_PROFILE {
                    return Err("scene profile mismatch".into());
                }
                Some(info.raster)
            }
        },
    };
    match info {
        None => {
            if !pixels.is_empty() {
                return Err("raster failure contains pixels".into());
            }
        }
        Some(info) => {
            if !mo_raster::accepts_profile(
                &info.profile,
                matches!(
                    mode,
                    Mode::SourceResourcePage(..) | Mode::SourcePlaybackPage(..)
                ),
            ) || info.width == 0
                || info.height == 0
                || info.width > 8192
                || info.height > 8192
                || u64::from(info.width) * u64::from(info.height) * 4 != pixels.len() as u64
                || info.byte_length.get() != pixels.len() as u64
                || info.sha256 != Digest::from_sha256(Sha256::digest(&pixels).into())
            {
                return Err("raster metadata does not match pixels".into());
            }
            super::artifact::publish(&pixels, output, |mut file| {
                if file.metadata()?.len() != info.byte_length.get() {
                    return Err("staged raster length differs".into());
                }
                let mut hash = Sha256::new();
                let mut buffer = [0; 16384];
                loop {
                    let n = file.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    hash.update(&buffer[..n]);
                }
                if Digest::from_sha256(hash.finalize().into()) != info.sha256 {
                    return Err("staged raster digest differs".into());
                }
                Ok(())
            })?;
        }
    }
    Ok(metadata)
}
