//! Typed, read-only OPC inspection; it makes no PPTX semantic/visual claim.
use mo_common::{ByteLength, Digest};
use mo_opc::{OpcError, Package, PackageLimits, ReaderAt, RelationshipSource, RelationshipTarget};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PackageInspectionResponse {
    Inspected { report: PackageReport },
    Error { error: PackageError },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageReport {
    pub format: PackageReportFormat,
    pub sha256: Digest,
    pub byte_length: ByteLength,
    /// True means signature-related parts/relationships exist, not signature validity.
    pub contains_signatures: bool,
    pub parts: Vec<PackagePart>,
    pub relationships: Vec<RelationshipGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PackageReportFormat {
    #[serde(rename = "musteroffice.opc-inspection/1")]
    V1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackagePart {
    pub name: String,
    pub content_type: String,
    pub byte_length: ByteLength,
    pub compressed_length: ByteLength,
    pub sha256: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationshipGroup {
    pub source: PackageSource,
    pub relationships: Vec<PackageRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PackageSource {
    Package,
    Part { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageRelationship {
    pub id: String,
    pub relationship_type: String,
    pub target: String,
    pub resolved: PackageTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PackageTarget {
    Internal {
        part: String,
        fragment: Option<String>,
    },
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PackageErrorCode {
    InputInvalid,
    LimitExceeded,
    Cancelled,
    Unsupported,
    PreservationConflict,
    ReadFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageError {
    pub code: PackageErrorCode,
    pub message: String,
}

impl PackageReport {
    /// Reuse a validated immutable package without reopening and decompressing
    /// it a second time merely to format the same inspection response.
    pub fn from_package<R: ReaderAt>(package: &Package<R>) -> Self {
        PackageReport {
            format: PackageReportFormat::V1,
            sha256: package.sha256().clone(),
            byte_length: ByteLength::new(package.byte_length()),
            contains_signatures: package.has_signatures(),
            parts: package
                .parts()
                .values()
                .map(|part| PackagePart {
                    name: part.name.to_string(),
                    content_type: part.content_type.clone(),
                    byte_length: ByteLength::new(part.byte_length),
                    compressed_length: ByteLength::new(part.compressed_length),
                    sha256: part.sha256.clone(),
                })
                .collect(),
            relationships: package
                .relationships()
                .iter()
                .map(|(source, entries)| RelationshipGroup {
                    source: match source {
                        RelationshipSource::Package => PackageSource::Package,
                        RelationshipSource::Part(name) => PackageSource::Part {
                            name: name.to_string(),
                        },
                    },
                    relationships: entries
                        .iter()
                        .map(|entry| PackageRelationship {
                            id: entry.id.clone(),
                            relationship_type: entry.relationship_type.clone(),
                            target: entry.target.clone(),
                            resolved: match &entry.resolved {
                                RelationshipTarget::Internal { part, fragment } => {
                                    PackageTarget::Internal {
                                        part: part.to_string(),
                                        fragment: fragment.clone(),
                                    }
                                }
                                RelationshipTarget::External => PackageTarget::External,
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// The caller grants the reader and owns cancellation. External links are never fetched.
pub fn inspect_package<R: ReaderAt>(
    reader: R,
    byte_length: u64,
    limits: PackageLimits,
    cancelled: &dyn Fn() -> bool,
) -> PackageInspectionResponse {
    match Package::open(reader, byte_length, limits, cancelled) {
        Ok(package) => PackageInspectionResponse::Inspected {
            report: PackageReport::from_package(&package),
        },
        Err(error) => PackageInspectionResponse::Error {
            error: PackageError {
                code: match &error {
                    OpcError::Cancelled => PackageErrorCode::Cancelled,
                    OpcError::Limit(_)
                    | OpcError::Xml {
                        source: mo_xml::XmlError::Limit(_),
                        ..
                    } => PackageErrorCode::LimitExceeded,
                    OpcError::Unsupported(_) => PackageErrorCode::Unsupported,
                    OpcError::Preservation(_) => PackageErrorCode::PreservationConflict,
                    OpcError::Io(error)
                        if !matches!(
                            error.kind(),
                            std::io::ErrorKind::InvalidData | std::io::ErrorKind::UnexpectedEof
                        ) =>
                    {
                        PackageErrorCode::ReadFailed
                    }
                    _ => PackageErrorCode::InputInvalid,
                },
                message: error.to_string(),
            },
        },
    }
}

pub fn inspect_package_json(bytes: &[u8]) -> String {
    package_response_json(&inspect_package(
        bytes,
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    ))
}

pub fn package_response_json(response: &PackageInspectionResponse) -> String {
    serde_json::to_string(response).expect("typed package inspection is JSON serializable")
}
