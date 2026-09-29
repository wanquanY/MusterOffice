//! Playback inputs derived from an inspected delivery, with no host authority.
use crate::{DeliveryAsset, DeliveryError, DeliverySettings, PreviewRequest, ReceivedDelivery};
use mo_common::{Digest, DocumentId, SlideId};
use mo_presentation_model::Size;
use mo_text::manifest::FontManifest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Byte identities and computation requests, not a grant, cached plan, or proof
/// that every animation in the source is supported. Playback preparation still
/// validates the selected source, explicit fonts and timing profile.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryPlaybackInputs {
    pub profile: String,
    pub document_id: DocumentId,
    /// Edited document provenance. Source playback bindings use source.sha256,
    /// which remains distinct from this document revision.
    pub revision: Digest,
    pub source: DeliveryAsset,
    pub font_bundle: Option<DeliveryAsset>,
    pub fonts: Option<FontManifest>,
    /// In presentation order; source part names come from the inspected OPC
    /// relationships and are never guessed from the page ordinal.
    pub pages: Vec<DeliveryPlaybackPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryPlaybackPage {
    pub page_id: SlideId,
    pub request: PreviewRequest,
}

pub(crate) struct PlaybackSource {
    pub source: DeliveryAsset,
    pub font_bundle: Option<DeliveryAsset>,
    pub settings: DeliverySettings,
    pub size: Size,
    pub pages: Vec<(SlideId, String)>,
}

impl ReceivedDelivery {
    /// Reuses the already inspected bytes/context. Does not reopen resources,
    /// rasterize pages, allocate pixels, create sessions or upgrade any claim.
    /// The receiver retains immutable asset readers and chooses its own Worker.
    pub fn playback_inputs(
        &self,
        width: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<DeliveryPlaybackInputs, DeliveryError> {
        crate::cancel(cancelled)?;
        let material = &self.playback;
        let viewport = crate::preview::viewport(material.size, width)?;
        let pages = material
            .pages
            .iter()
            .map(|(id, part)| {
                crate::cancel(cancelled)?;
                Ok(DeliveryPlaybackPage {
                    page_id: id.clone(),
                    request: crate::preview::request(
                        &material.settings,
                        &viewport,
                        &material.source.sha256,
                        part,
                    ),
                })
            })
            .collect::<Result<_, DeliveryError>>()?;
        crate::cancel(cancelled)?;
        Ok(DeliveryPlaybackInputs {
            profile: "delivery-playback-inputs-v1-draft".into(),
            document_id: self.report().document_id.clone(),
            revision: self.report().revision.clone(),
            source: material.source.clone(),
            font_bundle: material.font_bundle.clone(),
            fonts: material.settings.fonts.clone(),
            pages,
        })
    }
}
