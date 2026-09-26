use super::{NativeTimeline, read_slide_timing};
use crate::{
    PptxError,
    source::{SourceIndex, SourceLimits, SurfaceKind, inspect_source},
};
use mo_common::Digest;
use mo_opc::{PackageRead, PartName};
use mo_timeline::TimelineLimits;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTimingQuery {
    pub expected_source_sha256: Digest,
    pub slide: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTiming {
    pub source_sha256: Digest,
    pub slide: String,
    pub part_sha256: Digest,
    pub native: Option<NativeTimeline>,
}
/// Package identity and owning slide are validated before reading the native tree.
/// Projection is deliberately strict; it never rewrites unimplemented source data.
pub fn query(
    package: &dyn PackageRead,
    request: &SourceTimingQuery,
    limits: SourceLimits,
    timing_limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTiming, PptxError> {
    let index = inspect_source(package, limits, check)?;
    query_index(package, &index, request, limits, timing_limits, check)
}
/// Reuses a source index belonging to the same immutable package. Both identities
/// are checked; no source objects are manufactured by a timing projection.
pub fn query_index(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceTimingQuery,
    limits: SourceLimits,
    timing_limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTiming, PptxError> {
    crate::cancelled(check)?;
    if package.sha256() != &index.source_sha256
        || index.source_sha256 != request.expected_source_sha256
    {
        return Err(PptxError::SourceConflict("timing source digest".into()));
    }
    let slide = index
        .surfaces
        .get(&request.slide)
        .filter(|s| s.kind == SurfaceKind::Slide)
        .ok_or_else(|| crate::value("timing/slide", "slide part does not exist"))?;
    let known = slide.objects.iter().map(|s| s.native_id).collect();
    let part = PartName::new(&request.slide)?;
    let xml = package.read_part(&part, limits.package.xml.max_bytes as u64, check)?;
    let native = read_slide_timing(&xml, &known, limits.package.xml, timing_limits, check)?;
    Ok(SourceTiming {
        source_sha256: index.source_sha256.clone(),
        slide: request.slide.clone(),
        part_sha256: slide.sha256.clone(),
        native,
    })
}
