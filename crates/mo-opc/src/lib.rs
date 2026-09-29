//! Open Packaging Conventions graph and deterministic package I/O.
mod core_properties;
mod metadata;
mod names;
mod package;
mod read;
mod sink;
mod writer;
mod zip_structure;

pub use core_properties::{
    CORE_PROPERTIES_RELATIONSHIP, CORE_PROPERTIES_TYPE, CoreProperties, read_core_properties,
};
pub use metadata::{ContentTypes, Relationship, RelationshipSource, RelationshipTarget};
use mo_xml::XmlLimits;
pub use names::{PartName, relationship_part_name, relationship_source, resolve_internal_target};
pub use package::{Package, PartInfo};
pub use rawzip::ReaderAt;
pub use read::PackageRead;
pub use sink::{ResultSink, SealedOutput, VerifiedPackage};
use thiserror::Error;
pub use writer::{PackageBuilder, RewritePlan, WriteReceipt};

#[derive(Debug, Error)]
pub enum OpcError {
    #[error("invalid OPC part name: {0}")]
    PartName(String),
    #[error("invalid OPC structure: {0}")]
    Structure(String),
    #[error("OPC limit exceeded: {0}")]
    Limit(&'static str),
    #[error("OPC operation cancelled")]
    Cancelled,
    #[error("unsupported OPC input: {0}")]
    Unsupported(String),
    #[error("source preservation conflict: {0}")]
    Preservation(String),
    #[error("XML in {part}: {source}")]
    Xml {
        part: String,
        source: mo_xml::XmlError,
    },
    #[error(transparent)]
    Zip(#[from] rawzip::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy)]
pub struct PackageLimits {
    pub max_package_bytes: u64,
    pub max_part_bytes: u64,
    pub max_inflated_bytes: u64,
    pub max_parts: usize,
    pub max_compression_ratio: u64,
    pub max_relationships: usize,
    pub xml: XmlLimits,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_package_bytes: 1024 * 1024 * 1024,
            max_part_bytes: 1024 * 1024 * 1024,
            max_inflated_bytes: 2 * 1024 * 1024 * 1024,
            max_parts: 10_000,
            max_compression_ratio: 2_000,
            max_relationships: 100_000,
            xml: XmlLimits::default(),
        }
    }
}

pub(crate) const CONTENT_TYPES_NAME: &str = "[Content_Types].xml";
pub(crate) const RELS_TYPE: &str = "application/vnd.openxmlformats-package.relationships+xml";

pub(crate) fn check_cancel(cancelled: &dyn Fn() -> bool) -> Result<(), OpcError> {
    if cancelled() {
        Err(OpcError::Cancelled)
    } else {
        Ok(())
    }
}
