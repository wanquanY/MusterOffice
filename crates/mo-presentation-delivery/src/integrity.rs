use crate::*;
use mo_presentation_edit::SnapshotRecord;
use std::collections::BTreeMap;

/// Verify reference closure before the private candidate can leave computation.
/// This complements byte sealing; neither check constitutes host publication.
pub(crate) fn validate(
    bundle: &DeliveryBundle,
    snapshot: &SnapshotRecord,
) -> Result<(), DeliveryError> {
    let bad = || DeliveryError::Invalid("delivery reference closure");
    if bundle.version != "musteroffice.bundle/1-draft"
        || bundle.profile_id != PROFILE
        || bundle.document.document_id != snapshot.document.id
        || bundle.document.revision != snapshot.revision
        || bundle.previews.len() != snapshot.document.slide_order.len()
        || bundle.claims.len() != 5
    {
        return Err(bad());
    }
    let mut assets = BTreeMap::new();
    for asset in &bundle.assets {
        if assets.insert(&asset.id, asset).is_some() {
            return Err(bad());
        }
    }
    let pptx = assets.get(&bundle.pptx_asset_id).ok_or_else(bad)?;
    if pptx.role != AssetRole::Pptx || pptx.media_type != PPTX_MIME {
        return Err(bad());
    }
    let model = assets
        .get(&bundle.document.model_asset_id)
        .ok_or_else(bad)?;
    if model.role != AssetRole::EditableDocument || model.media_type != MODEL_MIME {
        return Err(bad());
    }
    for (sha, mime) in [
        (
            &bundle.versions.feature_registry_sha256,
            crate::registry::MIME,
        ),
        (
            &bundle.versions.font_profile_sha256,
            "application/vnd.musteroffice.font-profile+json",
        ),
    ] {
        if !bundle
            .assets
            .iter()
            .any(|a| a.role == AssetRole::Other && a.sha256 == *sha && a.media_type == mime)
        {
            return Err(bad());
        }
    }
    let mut image_ids = std::collections::BTreeSet::new();
    for (preview, page) in bundle.previews.iter().zip(&snapshot.document.slide_order) {
        let asset = assets.get(&preview.image_asset_id).ok_or_else(bad)?;
        if preview.page_id != *page
            || !image_ids.insert(&preview.image_asset_id)
            || asset.role != AssetRole::Preview
            || asset.media_type != "image/png"
            || !(1..=8192).contains(&preview.width)
            || !(1..=8192).contains(&preview.height)
        {
            return Err(bad());
        }
    }
    for kind in [
        ClaimKind::Structure,
        ClaimKind::Layout,
        ClaimKind::NativeEditability,
        ClaimKind::Playback,
        ClaimKind::TargetApplication,
    ] {
        let claims: Vec<_> = bundle.claims.iter().filter(|c| c.kind == kind).collect();
        if claims.len() != 1 {
            return Err(bad());
        }
        let claim = claims[0];
        if claim.subject_sha256 != pptx.sha256
            || claim.evidence_asset_ids.iter().any(|id| {
                assets
                    .get(id)
                    .is_none_or(|a| a.role != AssetRole::QualityReport)
            })
        {
            return Err(bad());
        }
        match claim.status {
            ClaimStatus::Passed | ClaimStatus::Failed => {
                if claim.evidence_asset_ids.is_empty() || claim.basis == ClaimBasis::None {
                    return Err(bad());
                }
                if kind == ClaimKind::TargetApplication
                    && claim.status == ClaimStatus::Passed
                    && claim.basis != ClaimBasis::ApplicationTest
                {
                    return Err(bad());
                }
            }
            ClaimStatus::NotProven | ClaimStatus::NotApplicable => {
                if claim.reason.as_ref().is_none_or(|s| s.trim().is_empty()) {
                    return Err(bad());
                }
            }
        }
    }
    Ok(())
}
