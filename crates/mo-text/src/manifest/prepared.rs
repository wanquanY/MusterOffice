//! Reusable verified manifest and paragraph/layout views for source compilers.
use super::*;
use crate::{
    flow::{OverflowPolicy, ParagraphLayoutRequest, ParagraphLayoutResult},
    geometry::{GeometryStyle, LineSpacing},
    resources::{FontResources, ResourceInput},
    scene::{ParagraphPathsRequest, ParagraphPathsResult},
};
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_unicode::bidi::ParagraphDirection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Borrowed content view: an operation does not need to duplicate its manifest
/// or resource bytes for each paragraph. Coordinates remain Unicode scalars.
#[derive(Clone, Copy)]
pub struct ManifestParagraphInput<'a> {
    pub text: &'a str,
    pub direction: ParagraphDirection,
    pub spans: &'a [itemize::StyleSpan],
    pub styles: &'a [ManifestTextStyle],
}
impl<'a> From<&'a ManifestParagraphRequest> for ManifestParagraphInput<'a> {
    fn from(q: &'a ManifestParagraphRequest) -> Self {
        Self {
            text: &q.text,
            direction: q.direction,
            spans: &q.spans,
            styles: &q.styles,
        }
    }
}
#[derive(Clone)]
pub struct ManifestLayoutInput<'a> {
    pub paragraph: ManifestParagraphInput<'a>,
    pub styles: &'a [GeometryStyle],
    pub strut_style: u32,
    pub spacing: LineSpacing,
    pub width: Emu,
    pub overflow: OverflowPolicy,
}
#[derive(Clone)]
pub struct ManifestFlowInput<'a> {
    pub paragraph: ManifestParagraphInput<'a>,
    pub styles: &'a [GeometryStyle],
    pub strut_style: u32,
    pub spacing: LineSpacing,
    pub widths: flow::LineWidths,
    pub overflow: OverflowPolicy,
    pub wrapping: flow::LineWrapping,
    pub hanging_punctuation: flow::HangingPunctuation,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestGeometryPaths {
    pub profile: String,
    pub bindings: Vec<ManifestStyleBinding>,
    pub geometry: scene::ParagraphGeometryPaths,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestLayoutResult {
    pub profile: String,
    pub bindings: Vec<ManifestStyleBinding>,
    pub layout: ParagraphLayoutResult,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestPathsResult {
    pub profile: String,
    pub bindings: Vec<ManifestStyleBinding>,
    pub paths: ParagraphPathsResult,
}

/// Borrows immutable author contracts and font bytes. Name records, unused
/// instances and resource identities are verified once before any text backend
/// operation. A mutable component instance is supplied separately per call.
pub struct PreparedManifest<'m, 'font> {
    manifest: &'m FontManifest,
    names: BTreeMap<&'m str, usize>,
    resources: FontResources<'font>,
    limits: ManifestLimits,
}
impl<'m, 'font> PreparedManifest<'m, 'font> {
    pub fn load(
        manifest: &'m FontManifest,
        bundle: &'font [u8],
        limits: ManifestLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TextError> {
        let names = binding::structure(manifest, limits, check)?;
        let resources =
            FontResources::load(&manifest.fonts, bundle, CascadeLimits::default(), check)?;
        binding::verify(manifest, resources.fonts(), resources.bindings(), check)?;
        cancelled(check)?;
        Ok(Self {
            manifest,
            names,
            resources,
            limits,
        })
    }
    /// One bounded residency scope for an entire native page/frame calculation.
    /// The scope releases every registered font even on cancellation or failure.
    pub fn font_session<'a>(
        &'a self,
        backend: &'a mut dyn backend::TextBackend,
    ) -> backend::FontSession<'a, 'font> {
        backend::FontSession::new(backend, self.resources.fonts())
    }
    /// Total immutable faces validated when loading this manifest. Repeated
    /// operations reuse them; per-operation FlowWork reports available faces,
    /// including faces not selected by that paragraph.
    pub fn verified_faces(&self) -> usize {
        self.resources.verified_faces()
    }
    /// Measure explicitly requested instances against an already verified font
    /// binding. Optional decoration metrics do not become mandatory line metrics.
    /// The metric protocol validates axes, batch limits and result identities.
    pub fn measure_instances(
        &self,
        font: u32,
        instances: &[crate::metrics::FontMetricsInstance],
        backend: &mut dyn backend::TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<crate::metrics::FontMetricsResult, TextError> {
        cancelled(check)?;
        if instances.len() > 256
            || instances
                .iter()
                .any(|i| i.metrics.len() > 28 || i.variations.len() > 64)
        {
            return Err(TextError::Limit(
                "manifest metric instances, values or axes",
            ));
        }
        let binding = self
            .resources
            .bindings()
            .get(font as usize)
            .ok_or(TextError::Invalid("manifest metric font binding"))?;
        let font = &self.resources.fonts()[*binding];
        crate::metrics::measure_verified(
            &crate::metrics::FontMetricsRequest {
                expected_sha256: font.metadata().sha256.clone(),
                face_index: font.metadata().face_index,
                instances: instances.to_vec(),
            },
            font,
            backend,
            check,
        )
    }
    /// Validate source compiler input, even if it is a later paragraph in an
    /// atomic frame operation. Does not invoke the component or hash fonts again.
    pub fn validate_paragraph(
        &self,
        input: ManifestParagraphInput<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<(), TextError> {
        let (q, _) = binding::paragraph(self.manifest, &self.names, input, self.limits, check)?;
        paragraph::prepare_items(&q, self.resources.bundle_length(), check)?;
        paragraph::validate_styles(&q, self.resources.fonts(), self.resources.bindings(), check)
    }
    pub fn paragraph_geometry(
        &self,
        input: ManifestFlowInput<'_>,
        bounds_tolerance: Fixed,
        backend: &mut dyn backend::TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<ManifestGeometryPaths, TextError> {
        cancelled(check)?;
        if input.styles.len() > 256 {
            return Err(TextError::Limit("manifest geometry styles"));
        }
        let (paragraph, bindings) = binding::paragraph(
            self.manifest,
            &self.names,
            input.paragraph,
            self.limits,
            check,
        )?;
        let geometry = scene::paragraph_geometry_using(
            &flow::FlowInput {
                paragraph: &paragraph,
                styles: input.styles,
                strut_style: input.strut_style,
                spacing: input.spacing,
                widths: input.widths,
                overflow: input.overflow,
                wrapping: input.wrapping,
                hanging_punctuation: input.hanging_punctuation,
            },
            bounds_tolerance,
            ResourceInput::Prepared(&self.resources),
            backend,
            check,
        )?;
        Ok(ManifestGeometryPaths {
            profile: "explicit-font-resource-manifest-draft-v1".into(),
            bindings,
            geometry,
        })
    }
    pub fn shape_paragraph(
        &self,
        input: ManifestParagraphInput<'_>,
        backend: &mut dyn backend::TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<ManifestParagraphResult, TextError> {
        let (q, bindings) =
            binding::paragraph(self.manifest, &self.names, input, self.limits, check)?;
        let shaping = paragraph::shape_paragraph_using(
            &q,
            ResourceInput::Prepared(&self.resources),
            backend,
            check,
        )?;
        Ok(ManifestParagraphResult {
            profile: "explicit-font-resource-manifest-draft-v1".into(),
            bindings,
            shaping,
        })
    }
    fn layout_request(
        &self,
        input: ManifestLayoutInput<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<(ParagraphLayoutRequest, Vec<ManifestStyleBinding>), TextError> {
        cancelled(check)?;
        if input.styles.len() > 256 {
            return Err(TextError::Limit("manifest geometry styles"));
        }
        let (paragraph, bindings) = binding::paragraph(
            self.manifest,
            &self.names,
            input.paragraph,
            self.limits,
            check,
        )?;
        Ok((
            ParagraphLayoutRequest {
                paragraph,
                styles: input.styles.to_vec(),
                strut_style: input.strut_style,
                spacing: input.spacing,
                width: input.width,
                overflow: input.overflow,
                hanging_punctuation: flow::HangingPunctuation::None,
                wrapping: flow::LineWrapping::Wrap,
            },
            bindings,
        ))
    }
    pub fn layout_paragraph(
        &self,
        input: ManifestLayoutInput<'_>,
        backend: &mut dyn backend::TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<ManifestLayoutResult, TextError> {
        let (q, bindings) = self.layout_request(input, check)?;
        let layout = flow::layout_with(
            &q,
            ResourceInput::Prepared(&self.resources),
            backend,
            check,
            geometry::evaluate,
        )?;
        Ok(ManifestLayoutResult {
            profile: "explicit-font-resource-manifest-draft-v1".into(),
            bindings,
            layout,
        })
    }
    pub fn paragraph_paths(
        &self,
        input: ManifestLayoutInput<'_>,
        bounds_tolerance: Fixed,
        backend: &mut dyn backend::TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<ManifestPathsResult, TextError> {
        let (layout, bindings) = self.layout_request(input, check)?;
        let paths = scene::paragraph_paths_using(
            &ParagraphPathsRequest {
                layout,
                bounds_tolerance,
            },
            ResourceInput::Prepared(&self.resources),
            backend,
            check,
        )?;
        Ok(ManifestPathsResult {
            profile: "explicit-font-resource-manifest-draft-v1".into(),
            bindings,
            paths,
        })
    }
}
