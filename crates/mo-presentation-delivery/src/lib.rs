//! Complete presentation delivery computation. No paths, network, process,
//! database, authority or implicit fonts. The host owns candidate publication.
mod artifact;
mod build;
mod contract;
mod integrity;
mod preview;
mod receive;
mod registry;
pub use artifact::{OutputStore, ProducedArtifact};
pub use build::{DeliveryCandidate, DeliveryInputs, build};
pub use contract::*;
use mo_opc::ReaderAt;
pub use preview::{PreviewFonts, PreviewInput, PreviewRenderer, PreviewRequest};
pub use receive::{
    DeliveryExpectation, DeliverySource, ReceiptInspection, ReceivedDelivery, inspect,
};

#[derive(Debug, thiserror::Error)]
pub enum DeliveryError {
    #[error("invalid delivery input: {0}")]
    Invalid(&'static str),
    #[error("delivery limit: {0}")]
    Limit(&'static str),
    #[error("delivery cancelled")]
    Cancelled,
    #[error("delivery storage: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Pptx(#[from] mo_pptx::PptxError),
    #[error(transparent)]
    Png(#[from] mo_image::png::PngError),
    #[error("delivery preview failed: {message}")]
    Preview {
        message: String,
        diagnostic: Option<Box<serde_json::Value>>,
    },
    #[error("invalid delivery serialization")]
    Serialization,
}
pub struct Content<'a> {
    pub reader: &'a dyn ReaderAt,
    pub byte_length: u64,
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), DeliveryError> {
    if check() {
        Err(DeliveryError::Cancelled)
    } else {
        Ok(())
    }
}
