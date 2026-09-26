//! Storage-independent access to an already validated, immutable OPC package.
use crate::{
    ContentTypes, OpcError, Package, PartInfo, PartName, ReaderAt, Relationship, RelationshipSource,
};
use mo_common::Digest;
use std::collections::BTreeMap;

mod sealed {
    pub trait Validated {}
    impl<R: crate::ReaderAt> Validated for crate::Package<R> {}
}

/// Read-only view of the graph and bytes verified by [`Package::open`].
///
/// This sealed interface lets format parsing and page preparation share one
/// implementation across host reader types. It neither allocates a wrapper nor
/// transfers the underlying resource. The host must keep its bytes immutable;
/// limits, cancellation and read failures still pass through the original
/// package. Only `Package` implements this interface, so callers cannot supply
/// substitute metadata without first verifying the actual package bytes.
pub trait PackageRead: sealed::Validated {
    fn parts(&self) -> &BTreeMap<PartName, PartInfo>;
    fn relationships(&self) -> &BTreeMap<RelationshipSource, Vec<Relationship>>;
    fn content_types(&self) -> &ContentTypes;
    fn sha256(&self) -> &Digest;
    fn byte_length(&self) -> u64;
    fn has_signatures(&self) -> bool;
    fn read_part(
        &self,
        name: &PartName,
        max_bytes: u64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, OpcError>;
}

impl<R: ReaderAt> PackageRead for Package<R> {
    fn parts(&self) -> &BTreeMap<PartName, PartInfo> {
        self.parts()
    }
    fn relationships(&self) -> &BTreeMap<RelationshipSource, Vec<Relationship>> {
        self.relationships()
    }
    fn content_types(&self) -> &ContentTypes {
        self.content_types()
    }
    fn sha256(&self) -> &Digest {
        self.sha256()
    }
    fn byte_length(&self) -> u64 {
        self.byte_length()
    }
    fn has_signatures(&self) -> bool {
        self.has_signatures()
    }
    fn read_part(
        &self,
        name: &PartName,
        max_bytes: u64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, OpcError> {
        self.read_part(name, max_bytes, cancelled)
    }
}
