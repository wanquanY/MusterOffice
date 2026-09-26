//! Immutable typed projection of author semantics. Imported declarations and
//! authored declarations use the same downstream inheritance/layout engines.
//! Its private index is a semantic plan, not an inspected physical OPC package.
mod bindings;
mod defaults;
mod geometry;
mod paint;
mod resources;
mod surface;
mod text;
mod theme;
use crate::{PptxError, cancelled, source::*, value as value_error};
pub use bindings::{LayoutPlan, MasterPlan, NativeBindings};
pub use defaults::{COLOR_SLOTS, ExportDefaults, FontDelivery};
use mo_common::{Digest, ResourceId};
use mo_presentation_model::{Document, ValidationLimits};
pub use resources::{NoResources, ResourceData, Resources};
use std::collections::BTreeMap;

pub struct AuthorPlan<'a> {
    document: &'a Document,
    defaults: &'a ExportDefaults,
    identity: Digest,
    bindings: NativeBindings,
    index: SourceIndex,
    image_references: BTreeMap<(String, String), ResourceId>,
}
impl<'a> AuthorPlan<'a> {
    pub fn new(
        document: &'a Document,
        defaults: &'a ExportDefaults,
        limits: ValidationLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        cancelled(check)?;
        let report = mo_presentation_model::validate(document, limits);
        if !report.is_valid() {
            return Err(PptxError::InvalidDocument(report));
        }
        if document
            .resources
            .values()
            .any(|r| r.kind == mo_presentation_model::ResourceKind::SourcePackage)
        {
            return Err(PptxError::Unsupported(
                "imported documents require a source-bound plan".into(),
            ));
        }
        for length in [document.page_size.width, document.page_size.height] {
            if !(914400..=51206400).contains(&length.get()) {
                return Err(value_error(
                    "pageSize",
                    "native slide dimension must be within 1..=56 inches",
                ));
            }
        }
        text::font(&defaults.font_family)?;
        text::centipoints(defaults.text_size, "defaults.textSize", 100, 400000)?;
        let identity =
            mo_common::digest("musteroffice.authored-native-plan/1", &(document, defaults))
                .map_err(|_| value_error("document", "semantic plan identity"))?;
        let bindings = bindings::plan(document)?;
        let mut ord = Ordinals { at: 0, check };
        let mut main_text = text::TextBuilder::new(&mut ord);
        main_text.defaults(defaults)?;
        let mut index = SourceIndex {
            text: main_text.catalog,
            compatibility_profile: "musteroffice.pml-source-mce/1".into(),
            main_compatibility: Default::default(),
            source_sha256: identity.clone(),
            byte_length: mo_common::ByteLength::new(0),
            main_part: "/ppt/presentation.xml".into(),
            main_content_type:
                "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"
                    .into(),
            contains_signatures: false,
            page_size: Some(document.page_size),
            slides: vec![],
            surfaces: BTreeMap::new(),
            themes: BTreeMap::new(),
            notices: vec![],
        };
        for (id, part) in &bindings.themes {
            let theme = theme::build(
                document,
                id.as_ref().map(|id| &document.themes[id]),
                defaults,
                &identity,
                &mut ord,
            )?;
            index.themes.insert(part.to_string(), theme);
        }
        let mut image_references = BTreeMap::new();
        surface::build(
            document,
            defaults,
            &bindings,
            &mut index,
            &mut image_references,
            &mut ord,
        )?;
        crate::source::resolve_projection(&mut index, check)?;
        cancelled(check)?;
        Ok(Self {
            document,
            defaults,
            identity,
            bindings,
            index,
            image_references,
        })
    }
    pub fn document(&self) -> &Document {
        self.document
    }
    pub fn defaults(&self) -> &ExportDefaults {
        self.defaults
    }
    pub fn identity(&self) -> &Digest {
        &self.identity
    }
    pub fn bindings(&self) -> &NativeBindings {
        &self.bindings
    }
    /// For compilation only. Ordinals identify typed declarations in this plan;
    /// they must never be interpreted as offsets into exported XML bytes.
    pub fn declarations(&self) -> &SourceIndex {
        &self.index
    }
    pub fn image_references(&self) -> &BTreeMap<(String, String), ResourceId> {
        &self.image_references
    }
}
struct Ordinals<'a> {
    at: u32,
    check: &'a dyn Fn() -> bool,
}
impl Ordinals<'_> {
    fn next(&mut self) -> Result<u32, PptxError> {
        cancelled(self.check)?;
        self.at = self
            .at
            .checked_add(1)
            .filter(|v| *v <= 1_000_000)
            .ok_or(PptxError::Limit("authored semantic declarations"))?;
        Ok(self.at)
    }
}
fn native<T: serde::de::DeserializeOwned>(value: String) -> Result<T, PptxError> {
    T::deserialize(serde::de::value::StringDeserializer::<
        serde::de::value::Error,
    >::new(value))
    .map_err(|e| value_error("native declaration", e.to_string()))
}
