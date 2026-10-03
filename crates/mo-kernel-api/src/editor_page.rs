//! One immutable rendered editor page per explicit owner. Preparation commits
//! only a complete raster; subsequent queries borrow its original interaction.
mod prepare;
mod types;
use crate::{PptxPageFailureCode, PptxResourcePageFailure, pptx_resource_page::request_failure};
use mo_common::Digest;
use mo_presentation_compile::source_editor_page::SourceEditorPage;
pub use types::*;

pub struct EditorPageBackends<'a> {
    pub decoder: &'a mut dyn mo_image::ImageDecoder,
    pub text: &'a mut dyn mo_text::backend::TextBackend,
    pub raster: &'a mut dyn mo_raster::RasterBackend,
}
#[derive(Default)]
pub struct EditorPageSession {
    current: Option<(Digest, SourceEditorPage)>,
}
fn failure(code: PptxPageFailureCode, message: &'static str) -> PptxResourcePageFailure {
    request_failure(code, message.into())
}
fn cancelled(check: &dyn Fn() -> bool) -> Result<(), PptxResourcePageFailure> {
    if check() {
        Err(failure(
            PptxPageFailureCode::Cancelled,
            "editor page cancelled",
        ))
    } else {
        Ok(())
    }
}
impl EditorPageSession {
    fn bound(&self, view: &Digest) -> Result<&SourceEditorPage, PptxResourcePageFailure> {
        self.current
            .as_ref()
            .filter(|(id, _)| id == view)
            .map(|(_, page)| page)
            .ok_or_else(|| {
                failure(
                    PptxPageFailureCode::SourceConflict,
                    "editor view is not current",
                )
            })
    }
    pub fn dispatch(
        &mut self,
        request: &EditorPageRequest,
        material: &[u8],
        fonts: &[u8],
        backends: Option<EditorPageBackends<'_>>,
        check: &dyn Fn() -> bool,
    ) -> (EditorPageResponse, Vec<u8>) {
        let result = (|| {
            cancelled(check)?;
            if material.len() > crate::MAX_INLINE_RESOURCE_BYTES
                || fonts.len() > crate::MAX_INLINE_FONT_BYTES
            {
                return Err(failure(
                    PptxPageFailureCode::LimitExceeded,
                    "editor page input bytes",
                ));
            }
            match request {
                EditorPageRequest::Prepare { request } => {
                    let backends = backends.ok_or_else(|| {
                        failure(
                            PptxPageFailureCode::ResourceRequired,
                            "editor page components required",
                        )
                    })?;
                    let (image, info) =
                        prepare::prepare(request, material, fonts, backends, check)?;
                    // The identity includes source/semantic plan, font manifest,
                    // viewport and sampling. Actual material hashes are verified
                    // by the shared preparation path before this commit point.
                    let view = mo_common::digest("musteroffice.editor-page-view/1", request)
                        .map_err(|_| {
                            failure(PptxPageFailureCode::InputInvalid, "editor view identity")
                        })?;
                    cancelled(check)?;
                    self.current = Some((view.clone(), image.page));
                    Ok((
                        EditorPageResponse::Prepared {
                            view,
                            info: Box::new(info),
                        },
                        image.pixels,
                    ))
                }
                _ if !material.is_empty() || !fonts.is_empty() => Err(failure(
                    PptxPageFailureCode::InputInvalid,
                    "bytes supplied to editor query or clear",
                )),
                EditorPageRequest::Query { view, queries } => {
                    let results = self
                        .bound(view)?
                        .query(queries, check)
                        .map_err(PptxResourcePageFailure::from)?;
                    Ok((
                        EditorPageResponse::Queried {
                            view: view.clone(),
                            results,
                        },
                        vec![],
                    ))
                }
                EditorPageRequest::Clear { view } => {
                    self.bound(view)?;
                    cancelled(check)?;
                    self.current = None;
                    Ok((EditorPageResponse::Cleared { view: view.clone() }, vec![]))
                }
            }
        })();
        result.unwrap_or_else(|error| {
            (
                EditorPageResponse::Error {
                    error: Box::new(error),
                },
                vec![],
            )
        })
    }
    pub fn dispatch_json(
        &mut self,
        request: &str,
        material: &[u8],
        fonts: &[u8],
        backends: Option<EditorPageBackends<'_>>,
        check: &dyn Fn() -> bool,
    ) -> (String, Vec<u8>) {
        let parsed = if request.len() > crate::MAX_REQUEST_BYTES {
            Err(failure(
                PptxPageFailureCode::LimitExceeded,
                "editor page request bytes",
            ))
        } else {
            mo_common::from_json_str(request)
                .map_err(|e| request_failure(PptxPageFailureCode::InputInvalid, e.to_string()))
        };
        let (response, pixels) = match parsed {
            Ok(request) => self.dispatch(&request, material, fonts, backends, check),
            Err(error) => (
                EditorPageResponse::Error {
                    error: Box::new(error),
                },
                vec![],
            ),
        };
        (
            serde_json::to_string(&response).expect("typed editor page response"),
            pixels,
        )
    }
}
