use super::*;
use mo_common::ResourceId;
use mo_presentation_model::ResourceKind;
use mo_text::manifest::FontManifest;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Context {
    format: String,
    pub settings: DeliverySettings,
    settings_digest: Digest,
    preview_renderer: RendererIdentity,
    model_asset_id: RequestId,
    pub resource_assets: BTreeMap<ResourceId, RequestId>,
    font_profile_asset_id: RequestId,
    pub font_bundle_asset_id: Option<RequestId>,
    registry_asset_id: RequestId,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FontProfile {
    format: String,
    manifest: Option<FontManifest>,
    bundle_asset_id: Option<RequestId>,
    bundle_sha256: Digest,
    byte_length: ByteLength,
}
pub(super) struct BoundContext {
    pub id: RequestId,
    pub value: Context,
}
fn same(a: &impl Serialize, b: &impl Serialize) -> Result<bool, DeliveryError> {
    Ok(
        serde_json::to_value(a).map_err(|_| DeliveryError::Serialization)?
            == serde_json::to_value(b).map_err(|_| DeliveryError::Serialization)?,
    )
}
pub(super) fn validate(
    input: &mut Inputs<'_>,
    bundle: &DeliveryBundle,
    snapshot: &SnapshotRecord,
    expected: &DeliveryExpectation,
) -> Result<BoundContext, DeliveryError> {
    let id = input.unique(AssetRole::Other, CONTEXT_MIME)?.id.clone();
    let context: Context = input.json(&id)?;
    if context.format != "musteroffice.presentation-context/1-draft"
        || context.model_asset_id != bundle.document.model_asset_id
        || context.settings_digest != expected.settings_digest
        || context.preview_renderer != expected.renderer
        || context.preview_renderer.profile
            != mo_presentation_compile::source_resource_page::PROFILE
        || context.resource_assets.len() != snapshot.document.resources.len()
    {
        return Err(DeliveryError::Invalid("delivery context binding"));
    }
    for (id, resource) in &snapshot.document.resources {
        cancel(input.check)?;
        let asset_id = context
            .resource_assets
            .get(id)
            .ok_or(DeliveryError::Invalid("model resource absent"))?;
        let role = match resource.kind {
            ResourceKind::Picture => AssetRole::Image,
            ResourceKind::Font => AssetRole::Font,
            ResourceKind::Audio => AssetRole::Audio,
            ResourceKind::Video => AssetRole::Video,
            ResourceKind::SourcePackage => AssetRole::Source,
            ResourceKind::EmbeddedWorkbook => AssetRole::Embedded,
            ResourceKind::Model3d => AssetRole::Model3d,
        };
        let asset = input.role(asset_id, role, &resource.media_type)?;
        if asset.sha256 != resource.sha256 {
            return Err(DeliveryError::Invalid("model resource digest"));
        }
    }
    let registry = input.role(
        &context.registry_asset_id,
        AssetRole::Other,
        crate::registry::MIME,
    )?;
    if registry.sha256 != bundle.versions.feature_registry_sha256
        || input.json::<serde_json::Value>(&registry.id)? != crate::registry::snapshot()
    {
        return Err(DeliveryError::Invalid("delivery registry version"));
    }
    let font = input.role(
        &context.font_profile_asset_id,
        AssetRole::Other,
        "application/vnd.musteroffice.font-profile+json",
    )?;
    if font.sha256 != bundle.versions.font_profile_sha256 {
        return Err(DeliveryError::Invalid("font profile digest"));
    }
    let font: FontProfile = input.json(&font.id)?;
    if font.format != "musteroffice.font-profile/1-draft"
        || font.bundle_asset_id != context.font_bundle_asset_id
        || !same(&font.manifest, &context.settings.fonts)?
        || font.byte_length.get() > input.limits.max_font_bytes
    {
        return Err(DeliveryError::Invalid("font context binding"));
    }
    if let Some(id) = &font.bundle_asset_id {
        let asset = input.role(id, AssetRole::Other, "application/octet-stream")?;
        if font.manifest.is_none()
            || asset.sha256 != font.bundle_sha256
            || asset.byte_length != font.byte_length
            || asset.byte_length.get() == 0
        {
            return Err(DeliveryError::Invalid("font bundle binding"));
        }
    } else if font.byte_length.get() != 0
        || font.bundle_sha256 != artifact::digest(&Vec::new(), 0, input.check)?
    {
        return Err(DeliveryError::Invalid("empty font bundle binding"));
    }
    if context
        .settings
        .input_digest(&font.bundle_sha256, &context.preview_renderer)
        .map_err(|_| DeliveryError::Serialization)?
        != expected.settings_digest
    {
        return Err(DeliveryError::Invalid("accepted delivery settings digest"));
    }
    Ok(BoundContext { id, value: context })
}
