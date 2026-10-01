use mo_common::{Digest, from_json_str};
pub use mo_image::DecodedImageInfo;
use mo_image::{ImageDecoder, ImageError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageDecodeRequest {
    pub source_sha256: Digest,
    /// Minimum oriented sample grid. Omission retains exact full-resolution decoding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_size: Option<mo_image::DecodeSize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImageFailureCode {
    InputInvalid,
    LimitExceeded,
    Unsupported,
    Cancelled,
    ComponentFailure,
    ComponentInvalid,
    HostFailure,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageFailure {
    pub code: ImageFailureCode,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ImageDecodeResponse {
    Decoded { info: DecodedImageInfo },
    Error { error: ImageFailure },
}
fn failure(e: ImageError) -> ImageFailure {
    use ImageFailureCode::*;
    let code = match e {
        ImageError::Invalid(_) | ImageError::Component(1) => InputInvalid,
        ImageError::Limit(_) | ImageError::Component(3) => LimitExceeded,
        ImageError::Component(5) => Unsupported,
        ImageError::Cancelled => Cancelled,
        ImageError::Component(_) => ComponentFailure,
        ImageError::ComponentInvalid(_) => ComponentInvalid,
        ImageError::Host(_) => HostFailure,
    };
    ImageFailure {
        code,
        message: e.to_string(),
    }
}
pub fn decode_image_json(
    input: &str,
    encoded: &[u8],
    decoder: &mut dyn ImageDecoder,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let result = if input.len() > crate::MAX_REQUEST_BYTES {
        Err(failure(ImageError::Limit("decode request bytes")))
    } else {
        from_json_str::<ImageDecodeRequest>(input)
            .map_err(|e| ImageFailure {
                code: ImageFailureCode::InputInvalid,
                message: e.to_string(),
            })
            .and_then(|q| {
                mo_image::decode_with_size(
                    encoded,
                    &q.source_sha256,
                    q.minimum_size,
                    decoder,
                    check,
                )
                .map_err(failure)
            })
    };
    let (response, pixels) = match result {
        Ok(image) => {
            let (info, pixels) = image.into_parts();
            (ImageDecodeResponse::Decoded { info }, pixels)
        }
        Err(error) => (ImageDecodeResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("bounded image decode metadata"),
        pixels,
    )
}
