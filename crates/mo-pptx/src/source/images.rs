//! Bind inherited image declarations to the immutable OPC graph. No decoding,
//! URL access, path lookup or implicit fallback to a different image source.
mod types;
use super::{SourceIndex, fill::resolve::*};
use crate::{PptxError, R, cancelled};
use mo_common::{ByteLength, Digest};
use mo_opc::{PackageRead, PartName, Relationship, RelationshipSource, RelationshipTarget};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
pub use types::*;

struct Resolver<'a> {
    package: &'a dyn PackageRead,
    limits: SourceImageLimits,
    check: &'a dyn Fn() -> bool,
    owners: BTreeMap<String, BTreeMap<&'a str, &'a Relationship>>,
    resources: Vec<EncodedImageResource>,
    resource_ids: BTreeMap<PartName, u32>,
    steps: usize,
    bytes: usize,
    bundle_bytes: u64,
}
impl<'a> Resolver<'a> {
    fn account(&mut self, bytes: usize) -> Result<(), PptxError> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|v| *v <= self.limits.max_metadata_bytes)
            .ok_or(PptxError::Limit("image reference metadata bytes"))?;
        Ok(())
    }
    fn relationship(
        &mut self,
        owner: &str,
        id: &str,
    ) -> Result<Option<&'a Relationship>, PptxError> {
        if !self.owners.contains_key(owner) {
            let part = PartName::new(owner)?;
            let relationships = self
                .package
                .relationships()
                .get(&RelationshipSource::Part(part));
            let mut refs = BTreeMap::new();
            for rel in relationships.into_iter().flatten() {
                cancelled(self.check)?;
                self.steps += 1;
                if self.steps > self.limits.max_relationship_steps {
                    return Err(PptxError::Limit("image relationship steps"));
                }
                refs.insert(rel.id.as_str(), rel);
            }
            self.account(owner.len())?;
            self.owners.insert(owner.to_owned(), refs);
        }
        Ok(self.owners[owner].get(id).copied())
    }
    fn resolve(
        &mut self,
        fill: EffectiveFill,
        redirects: Vec<FillRedirect>,
        selection: ImageSourceSelection,
    ) -> Result<SourceImageOutcome, PptxError> {
        let EffectiveFill::Image { declared_by, image } = fill else {
            return Ok(SourceImageOutcome::NotImage {});
        };
        let reference = match selection {
            ImageSourceSelection::EmbeddedSnapshot => &image.embed,
            ImageSourceSelection::LinkedSource => &image.link,
        };
        let unresolved = |issue| {
            Ok(SourceImageOutcome::UnresolvedReference {
                issue: Box::new(issue),
            })
        };
        if reference.value.is_empty() {
            return unresolved(ImageReferenceIssue::MissingSelectedReference {});
        }
        let Some(owner) = reference.declared_by.part() else {
            return unresolved(ImageReferenceIssue::MissingDeclaringPart {});
        };
        self.account(owner.len() + reference.value.len())?;
        let Some(rel) = self.relationship(owner, &reference.value)? else {
            return unresolved(ImageReferenceIssue::MissingRelationship {
                owner_part: owner.into(),
                relationship_id: reference.value.clone(),
            });
        };
        self.account(rel.target.len() + rel.relationship_type.len())?;
        let reference = SourceImageReference {
            declared_by: reference.declared_by.clone(),
            owner_part: owner.into(),
            relationship_id: reference.value.clone(),
            target_uri: rel.target.clone(),
        };
        if rel.relationship_type != format!("{R}/image") {
            return unresolved(ImageReferenceIssue::WrongRelationshipType {
                reference,
                relationship_type: rel.relationship_type.clone(),
            });
        }
        let binding = SourceImageBinding {
            declared_by,
            image: *image,
            redirects,
            reference,
        };
        match (&rel.resolved, selection) {
            (RelationshipTarget::External, ImageSourceSelection::LinkedSource) => {
                Ok(SourceImageOutcome::ExternalRequired {
                    binding: Box::new(binding),
                })
            }
            (
                RelationshipTarget::Internal {
                    part,
                    fragment: None,
                },
                ImageSourceSelection::EmbeddedSnapshot,
            ) => {
                let info = &self.package.parts()[part];
                if !info.content_type.to_ascii_lowercase().starts_with("image/") {
                    self.account(info.content_type.len())?;
                    return unresolved(ImageReferenceIssue::NonImageContentType {
                        reference: binding.reference,
                        content_type: info.content_type.clone(),
                    });
                }
                let resource = if let Some(id) = self.resource_ids.get(part) {
                    *id
                } else {
                    if self.resources.len() >= self.limits.max_resources {
                        return Err(PptxError::Limit("image resource count"));
                    }
                    if info.byte_length > self.limits.max_resource_bytes {
                        return Err(PptxError::Limit("encoded image bytes"));
                    }
                    let offset = self.bundle_bytes;
                    self.bundle_bytes = offset
                        .checked_add(info.byte_length)
                        .filter(|v| *v <= self.limits.max_bundle_bytes)
                        .ok_or(PptxError::Limit("encoded image bundle bytes"))?;
                    self.account(part.as_str().len() + info.content_type.len())?;
                    let id = self.resources.len() as u32;
                    self.resources.push(EncodedImageResource {
                        part: part.to_string(),
                        content_type: info.content_type.clone(),
                        sha256: info.sha256.clone(),
                        byte_length: ByteLength::new(info.byte_length),
                        offset: ByteLength::new(offset),
                    });
                    self.resource_ids.insert(part.clone(), id);
                    id
                };
                Ok(SourceImageOutcome::Available {
                    resource,
                    binding: Box::new(binding),
                })
            }
            (
                RelationshipTarget::Internal {
                    fragment: Some(_), ..
                },
                ImageSourceSelection::EmbeddedSnapshot,
            ) => unresolved(ImageReferenceIssue::Fragment {
                reference: binding.reference,
            }),
            _ => unresolved(ImageReferenceIssue::WrongTargetMode {
                reference: binding.reference,
            }),
        }
    }
}
pub fn query(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceImageQuery,
    limits: SourceImageLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceImageResources, PptxError> {
    query_on_page(
        package,
        index,
        request,
        &request.fill.surface,
        &request.fill.surface,
        limits,
        check,
    )
}
/// Page composition supplies validated drawing/background contexts while each
/// relationship continues to resolve against its own declaring part.
pub fn query_on_page(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceImageQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: SourceImageLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceImageResources, PptxError> {
    cancelled(check)?;
    if package.sha256() != &index.source_sha256
        || package.sha256() != &request.fill.expected_source_sha256
    {
        return Err(PptxError::SourceConflict(
            "image package/index digest".into(),
        ));
    }
    let fills = super::fill::resolve::query_on_page(
        index,
        &request.fill,
        drawing_surface,
        background_surface,
        limits.fills,
        check,
    )?;
    let mut resolver = Resolver {
        package,
        limits,
        check,
        owners: BTreeMap::new(),
        resources: vec![],
        resource_ids: BTreeMap::new(),
        steps: 0,
        bytes: 0,
        bundle_bytes: 0,
    };
    let mut targets = Vec::with_capacity(fills.targets.len());
    for item in fills.targets {
        cancelled(check)?;
        let outcome = match item.outcome {
            FillOutcome::Unresolved { reason } => SourceImageOutcome::UnresolvedFill {
                reason: Box::new(reason),
            },
            FillOutcome::Resolved { fill, redirects } => {
                resolver.resolve(*fill, redirects, request.selection)?
            }
        };
        targets.push(SourceImageResult {
            target: item.target,
            outcome,
        });
    }
    cancelled(check)?;
    Ok(SourceImageResources {
        source_sha256: index.source_sha256.clone(),
        surface: request.fill.surface.clone(),
        selection: request.selection,
        targets,
        resources: resolver.resources,
        bundle_byte_length: ByteLength::new(resolver.bundle_bytes),
    })
}

/// Extract only advertised internal resources. The returned catalog still
/// exposes any unresolved/external targets; it is not a complete rendered page.
/// Revalidate every descriptor before I/O, then verify each actual payload hash.
pub fn extract(
    package: &dyn PackageRead,
    resources: &SourceImageResources,
    limits: SourceImageLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<u8>, PptxError> {
    cancelled(check)?;
    if package.sha256() != &resources.source_sha256 {
        return Err(PptxError::SourceConflict("image extraction source".into()));
    }
    if resources.resources.len() > limits.max_resources
        || resources.bundle_byte_length.get() > limits.max_bundle_bytes
    {
        return Err(PptxError::Limit("image extraction budget"));
    }
    let mut total = 0u64;
    let mut parts = std::collections::BTreeSet::new();
    for r in &resources.resources {
        cancelled(check)?;
        let part = PartName::new(&r.part)?;
        let Some(info) = package.parts().get(&part) else {
            return Err(PptxError::SourceConflict("missing image part".into()));
        };
        if !parts.insert(part)
            || r.offset.get() != total
            || r.byte_length.get() != info.byte_length
            || r.sha256 != info.sha256
            || r.content_type != info.content_type
            || !info.content_type.to_ascii_lowercase().starts_with("image/")
        {
            return Err(PptxError::SourceConflict("image resource binding".into()));
        }
        if info.byte_length > limits.max_resource_bytes {
            return Err(PptxError::Limit("encoded image bytes"));
        }
        total = total
            .checked_add(info.byte_length)
            .filter(|n| *n <= limits.max_bundle_bytes)
            .ok_or(PptxError::Limit("encoded image bundle bytes"))?;
    }
    if total != resources.bundle_byte_length.get() {
        return Err(PptxError::SourceConflict("image bundle length".into()));
    }
    let mut out = Vec::new();
    out.try_reserve_exact(
        usize::try_from(total).map_err(|_| PptxError::Limit("image bundle address range"))?,
    )
    .map_err(|_| PptxError::Limit("image bundle allocation"))?;
    for r in &resources.resources {
        let bytes =
            package.read_part(&PartName::new(&r.part)?, limits.max_resource_bytes, check)?;
        let mut hash = Sha256::new();
        for block in bytes.chunks(16384) {
            cancelled(check)?;
            hash.update(block);
        }
        if Digest::from_sha256(hash.finalize().into()) != r.sha256 {
            return Err(PptxError::SourceConflict("image payload changed".into()));
        }
        out.extend_from_slice(&bytes);
    }
    cancelled(check)?;
    Ok(out)
}
