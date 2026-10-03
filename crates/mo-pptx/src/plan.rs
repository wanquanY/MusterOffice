//! One immutable input plan for native writing and preview. Mixed source-backed
//! documents are composed by the format owner and retain the original graph.
mod composed;
use crate::{AuthorPlan, ExportDefaults, PptxError, Resources};
pub use composed::ComposedPlan;
use mo_common::Digest;
use mo_opc::{Package, PackageLimits, ReaderAt};
use mo_presentation_model::{Document, ValidationLimits};
use mo_presentation_source::source::{SourceIndex, document::SourcePlan};

pub enum PresentationPlan<'a> {
    Author(AuthorPlan<'a>),
    Retained {
        plan: Box<SourcePlan<'a>>,
        source: Package<&'a dyn ReaderAt>,
    },
    Composed(Box<ComposedPlan>),
}
impl<'a> PresentationPlan<'a> {
    pub fn new(
        document: &'a Document,
        defaults: &'a ExportDefaults,
        resources: &'a (impl Resources + ?Sized),
        document_limits: ValidationLimits,
        package_limits: PackageLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        if document
            .source_bindings
            .as_ref()
            .is_some_and(|b| b.slides.len() != document.slides.len())
        {
            return Ok(Self::Composed(Box::new(ComposedPlan::new(
                document,
                defaults,
                resources,
                document_limits,
                package_limits,
                check,
            )?)));
        }
        use mo_presentation_source::plan::PresentationPlan as Base;
        Ok(
            match Base::new(
                document,
                defaults,
                resources,
                document_limits,
                package_limits,
                check,
            )? {
                Base::Author(plan) => Self::Author(plan),
                Base::Retained { plan, source } => Self::Retained { plan, source },
            },
        )
    }
    pub fn identity(&self) -> &Digest {
        match self {
            Self::Author(p) => p.identity(),
            Self::Retained { plan, .. } => plan.identity(),
            Self::Composed(p) => p.identity(),
        }
    }
    pub fn declarations(&self) -> &SourceIndex {
        match self {
            Self::Author(p) => p.declarations(),
            Self::Retained { plan, .. } => plan.declarations(),
            Self::Composed(p) => p.declarations(),
        }
    }
}
