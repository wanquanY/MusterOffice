//! Shared input plan for one document domain. Native retention is an immutable
//! source binding, not a second editable document or transaction mechanism.
use crate::{
    PptxError,
    author::{AuthorPlan, ExportDefaults, Resources},
    source::{SourceIndex, SourceLimits, document::SourcePlan},
};
use mo_common::Digest;
use mo_opc::{Package, PackageLimits, ReaderAt};
use mo_presentation_model::{Document, ValidationLimits};

pub enum PresentationPlan<'a> {
    Author(AuthorPlan<'a>),
    Retained {
        plan: Box<SourcePlan<'a>>,
        source: Package<&'a dyn ReaderAt>,
    },
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
        if let Some(b) = &document.source_bindings {
            let report = mo_presentation_model::validate(document, document_limits);
            if !report.is_valid() {
                return Err(PptxError::InvalidDocument(report));
            }
            let data = resources.open(&b.resource)?;
            let source = Package::open(data.reader, data.byte_length, package_limits, check)?;
            let plan = SourcePlan::new(
                document,
                &source,
                SourceLimits {
                    package: package_limits,
                    ..Default::default()
                },
                check,
            )?;
            Ok(Self::Retained {
                plan: Box::new(plan),
                source,
            })
        } else {
            Ok(Self::Author(AuthorPlan::new(
                document,
                defaults,
                document_limits,
                check,
            )?))
        }
    }
    pub fn identity(&self) -> &Digest {
        match self {
            Self::Author(p) => p.identity(),
            Self::Retained { plan, .. } => plan.identity(),
        }
    }
    pub fn declarations(&self) -> &SourceIndex {
        match self {
            Self::Author(p) => p.declarations(),
            Self::Retained { plan, .. } => plan.declarations(),
        }
    }
}
