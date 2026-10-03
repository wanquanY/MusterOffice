//! Bounded portable input derivation; actual playback uses the existing owners.
use crate::{DeliveryInspectRequest, PptxFailure, delivery};
use mo_opc::ReaderAt;
use mo_presentation_delivery::{DeliveryError, DeliveryPlaybackInputs};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryPlaybackRequest {
    pub delivery: DeliveryInspectRequest,
    /// Pixel width; height/scale preserve the inspected document's geometry.
    pub width: u32,
    /// When present, fit within both pixel dimensions using kernel geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum DeliveryPlaybackResponse {
    Prepared { inputs: Box<DeliveryPlaybackInputs> },
    Error { error: PptxFailure },
}

fn run<R: ReaderAt>(
    input: &str,
    reader: &R,
    length: u64,
    check: &dyn Fn() -> bool,
) -> Result<DeliveryPlaybackInputs, DeliveryError> {
    if check() {
        return Err(DeliveryError::Cancelled);
    }
    if input.len() > crate::MAX_REQUEST_BYTES {
        return Err(DeliveryError::Limit("inline delivery request"));
    }
    let request: DeliveryPlaybackRequest = mo_common::from_json_str(input)
        .map_err(|_| DeliveryError::Invalid("delivery playback JSON"))?;
    if request.width == 0 || (request.height.is_none() && request.width > 8192) {
        return Err(DeliveryError::Invalid("playback width"));
    }
    let delivery = delivery::inspect_request(request.delivery, reader, length, check)?;
    match request.height {
        Some(height) => delivery.playback_inputs_fit(request.width, height, check),
        None => delivery.playback_inputs(request.width, check),
    }
}

/// Native products can reuse ReceivedDelivery::playback_inputs without inline
/// packing or a second inspection. This bridge serves explicit portable inputs.
pub fn prepare_delivery_playback_at<R: ReaderAt>(
    input: &str,
    reader: &R,
    length: u64,
    check: &dyn Fn() -> bool,
) -> DeliveryPlaybackResponse {
    match run(input, reader, length, check) {
        Ok(inputs) => DeliveryPlaybackResponse::Prepared {
            inputs: Box::new(inputs),
        },
        Err(error) => DeliveryPlaybackResponse::Error {
            error: delivery::failure(error),
        },
    }
}

pub fn prepare_delivery_playback_json(input: &str, bytes: &[u8]) -> String {
    serde_json::to_string(&prepare_delivery_playback_at(
        input,
        &bytes,
        bytes.len() as u64,
        &|| false,
    ))
    .expect("typed delivery playback response")
}
