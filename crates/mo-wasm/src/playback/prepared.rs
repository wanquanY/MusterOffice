//! Explicit frame ownership across JS yields. Rust never suspends a callback.
use super::*;
use mo_kernel_api::{PlaybackSessionResponse as A, PptxPlaybackSessionResponse as S};

#[wasm_bindgen(typescript_custom_section)]
const INTERFACE: &str = r#"
export interface SteppedRasterComponent {
    beginRaster(frame: Uint32Array, images?: Uint8Array): unknown;
}
"#;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "SteppedRasterComponent")]
    pub type SteppedRasterComponent;
    #[wasm_bindgen(method, catch, js_name = beginRaster)]
    fn begin_paths(this: &SteppedRasterComponent, frame: &[u32]) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(method, catch, js_name = beginRaster)]
    fn begin_images(
        this: &SteppedRasterComponent,
        frame: &[u32],
        images: &[u8],
    ) -> Result<JsValue, JsValue>;
}
enum Frame {
    Author(Box<mo_kernel_api::PreparedPlaybackRender>),
    Source(Box<mo_kernel_api::PreparedPptxPlaybackRender>),
}
impl Frame {
    fn words(&self) -> &[u32] {
        match self {
            Self::Author(frame) => frame.words(),
            Self::Source(frame) => frame.words(),
        }
    }
}
#[wasm_bindgen]
pub struct PreparedPlaybackRaster {
    frame: Option<Frame>,
    failure: String,
    started: bool,
}
#[wasm_bindgen]
impl PreparedPlaybackRaster {
    /// Empty on success; otherwise the existing typed session error response.
    #[wasm_bindgen(getter)]
    pub fn failure(&self) -> String {
        self.failure.clone()
    }
    /// The component copies borrowed Rust memory once, before this call returns.
    /// No temporary full-image JS copy or cross-yield Rust memory borrow.
    pub fn begin(&mut self, component: &SteppedRasterComponent) -> Result<JsValue, JsValue> {
        if self.started {
            return Err(JsValue::from_str("frame execution already started"));
        }
        let frame = self
            .frame
            .as_ref()
            .ok_or_else(|| JsValue::from_str("frame preparation failed"))?;
        self.started = true;
        match frame {
            Frame::Author(frame) => component.begin_paths(frame.words()),
            Frame::Source(frame) => component.begin_images(frame.words(), frame.images()),
        }
    }
}
#[wasm_bindgen]
pub struct CompletedPlaybackRaster {
    metadata: String,
    pixels: Vec<u8>,
    invalidate: bool,
}
#[wasm_bindgen]
impl CompletedPlaybackRaster {
    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> String {
        self.metadata.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn invalidates_backend(&self) -> bool {
        self.invalidate
    }
    pub fn take_pixels(self) -> Vec<u8> {
        self.pixels
    }
}
fn decode(
    prepared: &PreparedPlaybackRaster,
    reply: &crate::raster::Reply,
) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
    match prepared.frame.as_ref() {
        Some(frame) => crate::raster::read_reply(reply, frame.words()),
        None => Err(mo_raster::RasterError::Invalid("frame preparation failed")),
    }
}
fn invalidates(reply: &Result<mo_raster::BackendReply, mo_raster::RasterError>) -> bool {
    match reply {
        Ok(reply) => reply.check_status().is_err_and(|e| e.invalidates_backend()),
        Err(error) => error.invalidates_backend(),
    }
}
#[wasm_bindgen]
impl PlaybackSession {
    pub fn prepare_render(&mut self, request: &str) -> PreparedPlaybackRaster {
        match self.inner.prepare_render_json(request, &|| false) {
            Ok(frame) => PreparedPlaybackRaster {
                frame: Some(Frame::Author(Box::new(frame))),
                failure: String::new(),
                started: false,
            },
            Err(error) => PreparedPlaybackRaster {
                frame: None,
                failure: serde_json::to_string(&A::Error { error }).expect("typed error"),
                started: false,
            },
        }
    }
    pub fn complete_render(
        &mut self,
        prepared: PreparedPlaybackRaster,
        reply: &crate::raster::Reply,
    ) -> CompletedPlaybackRaster {
        let reply = decode(&prepared, reply);
        let mut invalidate = invalidates(&reply);
        let result = match prepared.frame {
            Some(Frame::Author(frame)) if prepared.started => {
                self.inner.complete_render_result(*frame, reply, &|| false)
            }
            _ => Err(mo_kernel_api::PlaybackCompletionFailure {
                error: mo_kernel_api::PlaybackSessionFailure::Session {
                    code: mo_kernel_api::PlaybackSessionFailureCode::BindingConflict,
                    message: "frame not started for author playback".into(),
                },
                invalidate_backend: invalidate,
            }),
        };
        let (response, pixels) = match result {
            Ok(image) => (
                A::Rendered {
                    info: Box::new(image.info),
                },
                image.pixels,
            ),
            Err(error) => {
                invalidate |= error.invalidate_backend;
                (A::Error { error: error.error }, vec![])
            }
        };
        CompletedPlaybackRaster {
            metadata: serde_json::to_string(&response).expect("typed frame"),
            pixels,
            invalidate,
        }
    }
}
#[wasm_bindgen]
impl PptxPlaybackSession {
    pub fn prepare_render(&mut self, request: &str) -> PreparedPlaybackRaster {
        match self.inner.prepare_render_json(request, &|| false) {
            Ok(frame) => PreparedPlaybackRaster {
                frame: Some(Frame::Source(Box::new(frame))),
                failure: String::new(),
                started: false,
            },
            Err(error) => PreparedPlaybackRaster {
                frame: None,
                failure: serde_json::to_string(&S::Error { error }).expect("typed error"),
                started: false,
            },
        }
    }
    pub fn complete_render(
        &mut self,
        prepared: PreparedPlaybackRaster,
        reply: &crate::raster::Reply,
    ) -> CompletedPlaybackRaster {
        let reply = decode(&prepared, reply);
        let mut invalidate = invalidates(&reply);
        let result = match prepared.frame {
            Some(Frame::Source(frame)) if prepared.started => {
                self.inner.complete_render_result(*frame, reply, &|| false)
            }
            _ => Err(mo_kernel_api::PlaybackCompletionFailure {
                error: mo_kernel_api::PptxPlaybackSessionFailure::Session {
                    code: mo_kernel_api::PlaybackSessionFailureCode::BindingConflict,
                    message: "frame not started for source playback".into(),
                },
                invalidate_backend: invalidate,
            }),
        };
        let (response, pixels) = match result {
            Ok((info, pixels)) => (
                S::Rendered {
                    info: Box::new(info),
                },
                pixels,
            ),
            Err(error) => {
                invalidate |= error.invalidate_backend;
                (S::Error { error: error.error }, vec![])
            }
        };
        CompletedPlaybackRaster {
            metadata: serde_json::to_string(&response).expect("typed frame"),
            pixels,
            invalidate,
        }
    }
}
