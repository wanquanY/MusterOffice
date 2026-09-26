//! A typed image graph shared by source packages and immutable author plans.
//! This is deliberately narrower than PackageRead: logical author addresses
//! are never presented as inspected OPC bytes or a sealed package proof.
use super::*;
use crate::author::{AuthorPlan, ResourceData, Resources};
use mo_common::ResourceId;

mod sealed {
    pub trait Sealed {}
}
pub struct ImagePart<'a> {
    pub content_type: &'a str,
    pub sha256: &'a Digest,
    pub byte_length: u64,
}
pub trait ImageInput: sealed::Sealed {
    /// Package digest for source input, domain-separated semantic plan digest
    /// for author input. Both bind the declarations and relationship graph.
    fn identity(&self) -> &Digest;
    fn relationships(&self, owner: &PartName) -> Option<&[Relationship]>;
    fn part(&self, part: &PartName) -> Option<ImagePart<'_>>;
    fn read(
        &self,
        part: &PartName,
        max: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, PptxError>;
}

pub struct PackageImages<'a>(pub &'a dyn PackageRead);
impl sealed::Sealed for PackageImages<'_> {}
impl ImageInput for PackageImages<'_> {
    fn identity(&self) -> &Digest {
        self.0.sha256()
    }
    fn relationships(&self, owner: &PartName) -> Option<&[Relationship]> {
        self.0
            .relationships()
            .get(&RelationshipSource::Part(owner.clone()))
            .map(Vec::as_slice)
    }
    fn part(&self, part: &PartName) -> Option<ImagePart<'_>> {
        self.0.parts().get(part).map(|p| ImagePart {
            content_type: &p.content_type,
            sha256: &p.sha256,
            byte_length: p.byte_length,
        })
    }
    fn read(
        &self,
        part: &PartName,
        max: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, PptxError> {
        Ok(self.0.read_part(part, max, check)?)
    }
}

struct AuthorImage<'a> {
    resource: &'a mo_presentation_model::Resource,
    data: ResourceData<'a>,
}
pub struct AuthorImages<'a> {
    identity: &'a Digest,
    relationships: BTreeMap<PartName, Vec<Relationship>>,
    parts: BTreeMap<PartName, AuthorImage<'a>>,
}
impl<'a> AuthorImages<'a> {
    /// Bind once per document. Encoded bytes are read on demand and verified
    /// before decode. Resource IDs, hashes and media types come from the plan.
    pub fn new(
        plan: &'a AuthorPlan<'_>,
        resources: &'a (impl Resources + ?Sized),
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        let mut parts = BTreeMap::new();
        for (id, part) in &plan.bindings().images {
            cancelled(check)?;
            parts.insert(
                part.clone(),
                AuthorImage {
                    resource: &plan.document().resources[id],
                    data: resources.open(id)?,
                },
            );
        }
        let mut relationships: BTreeMap<PartName, Vec<Relationship>> = BTreeMap::new();
        for ((owner, id), resource) in plan.image_references() {
            cancelled(check)?;
            let owner = PartName::new(owner)?;
            relationships
                .entry(owner.clone())
                .or_default()
                .push(Relationship::new(
                    &RelationshipSource::Part(owner),
                    id.clone(),
                    format!("{R}/image"),
                    plan.bindings().images[resource].to_string(),
                    false,
                )?);
        }
        Ok(Self {
            identity: plan.identity(),
            relationships,
            parts,
        })
    }
    pub fn resource(&self, part: &PartName) -> Option<&ResourceId> {
        self.parts.get(part).map(|p| &p.resource.id)
    }
}
impl sealed::Sealed for AuthorImages<'_> {}
impl ImageInput for AuthorImages<'_> {
    fn identity(&self) -> &Digest {
        self.identity
    }
    fn relationships(&self, owner: &PartName) -> Option<&[Relationship]> {
        self.relationships.get(owner).map(Vec::as_slice)
    }
    fn part(&self, part: &PartName) -> Option<ImagePart<'_>> {
        self.parts.get(part).map(|p| ImagePart {
            content_type: &p.resource.media_type,
            sha256: &p.resource.sha256,
            byte_length: p.data.byte_length,
        })
    }
    fn read(
        &self,
        part: &PartName,
        max: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, PptxError> {
        cancelled(check)?;
        let p = self
            .parts
            .get(part)
            .ok_or_else(|| PptxError::SourceConflict("author image binding".into()))?;
        if p.data.byte_length > max {
            return Err(PptxError::Limit("encoded image bytes"));
        }
        let length = usize::try_from(p.data.byte_length)
            .map_err(|_| PptxError::Limit("image address range"))?;
        let mut out = Vec::new();
        out.try_reserve_exact(length)
            .map_err(|_| PptxError::Limit("image allocation"))?;
        out.resize(length, 0);
        let mut hash = Sha256::new();
        for (i, block) in out.chunks_mut(16384).enumerate() {
            cancelled(check)?;
            p.data
                .reader
                .read_exact_at(block, (i * 16384) as u64)
                .map_err(mo_opc::OpcError::from)?;
            hash.update(block);
        }
        if Digest::from_sha256(hash.finalize().into()) != p.resource.sha256 {
            return Err(PptxError::SourceConflict(
                "author image payload changed".into(),
            ));
        }
        cancelled(check)?;
        Ok(out)
    }
}
