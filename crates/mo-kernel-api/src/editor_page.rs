//! One immutable rendered editor page per explicit owner. Preparation commits
//! only a complete raster; subsequent queries borrow its original interaction.
mod catalog;
mod inspect;
mod prepare;
mod text_identity;
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
pub enum EditorPageComponents<'a> {
    All(EditorPageBackends<'a>),
    Raster(&'a mut dyn mo_raster::RasterBackend),
}
struct CurrentPage {
    view: Digest,
    page: SourceEditorPage,
    objects: std::collections::BTreeMap<(String, u32), u32>,
}
#[derive(Default)]
pub struct EditorPageSession {
    current: Option<CurrentPage>,
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
fn source_error(error: mo_pptx::PptxError) -> PptxResourcePageFailure {
    mo_presentation_compile::source_page::SourcePageError::Source(error).into()
}
impl EditorPageSession {
    fn bound(&self, view: &Digest) -> Result<&CurrentPage, PptxResourcePageFailure> {
        self.current
            .as_ref()
            .filter(|page| &page.view == view)
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
        backends: Option<EditorPageComponents<'_>>,
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
                EditorPageRequest::Inspect { input } => {
                    if !fonts.is_empty() {
                        return Err(failure(
                            PptxPageFailureCode::InputInvalid,
                            "fonts supplied to editor document inspection",
                        ));
                    }
                    let info = inspect::inspect(input, material, check)?;
                    cancelled(check)?;
                    Ok((
                        EditorPageResponse::Inspected {
                            info: Box::new(info),
                        },
                        vec![],
                    ))
                }
                EditorPageRequest::Prepare { request } => {
                    let backends = match backends {
                        Some(EditorPageComponents::All(b)) => Some(b),
                        _ => None,
                    }
                    .ok_or_else(|| {
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
                    let objects = info
                        .objects
                        .iter()
                        .enumerate()
                        .map(|(i, o)| ((o.object.part.clone(), o.object.native_id), i as u32))
                        .collect();
                    self.current = Some(CurrentPage {
                        view: view.clone(),
                        page: image.page,
                        objects,
                    });
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
                    "bytes supplied to non-preparation editor command",
                )),
                EditorPageRequest::Query { view, queries } => {
                    let results = self
                        .bound(view)?
                        .page
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
                EditorPageRequest::Pick { view, queries } => {
                    let current = self.bound(view)?;
                    let raster = match backends {
                        Some(EditorPageComponents::All(b)) => b.raster,
                        Some(EditorPageComponents::Raster(r)) => r,
                        None => {
                            return Err(failure(
                                PptxPageFailureCode::ResourceRequired,
                                "editor picking component required",
                            ));
                        }
                    };
                    let picked = current.page.pick(queries, raster, check)?;
                    let mut results = Vec::with_capacity(picked.len());
                    for result in picked {
                        cancelled(check)?;
                        let mut hits = Vec::with_capacity(result.hits.len());
                        for hit in result.hits {
                            cancelled(check)?;
                            let object = *current
                                .objects
                                .get(&(hit.object.part, hit.object.native_id))
                                .ok_or_else(|| {
                                    failure(
                                        PptxPageFailureCode::InputInvalid,
                                        "editor picking object identity",
                                    )
                                })?;
                            hits.push(EditorObjectHit {
                                object,
                                kind: hit.kind,
                                text_frame: hit.text_frame,
                            });
                        }
                        results.push(EditorPickResult {
                            hits,
                            truncated: result.truncated,
                        });
                    }
                    Ok((
                        EditorPageResponse::Picked {
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
        backends: Option<EditorPageComponents<'_>>,
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
