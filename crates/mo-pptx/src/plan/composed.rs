//! Retained fields and independent authored pages share one editable document.
//! The original source projection is still checked in full before composition.
mod graft;
use crate::*;
use mo_common::{Digest, ObjectId};
use mo_opc::{Package, PackageLimits, ResultSink, RewritePlan, VerifiedPackage};
use mo_presentation_model::{Document, SourceBindingProfile, ValidationLimits};
use mo_presentation_source::source::{SourceIndex, SourceLimits, document::SourcePlan};
use std::collections::BTreeMap;

pub struct ComposedPlan {
    package: VerifiedPackage<Vec<u8>>,
    declarations: SourceIndex,
    object_bindings: BTreeMap<(String, u32), ObjectId>,
}
impl ComposedPlan {
    pub(super) fn new(
        document: &Document,
        defaults: &ExportDefaults,
        resources: &(impl Resources + ?Sized),
        document_limits: ValidationLimits,
        package_limits: PackageLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        cancelled(check)?;
        let report = mo_presentation_model::validate(document, document_limits);
        if !report.is_valid() {
            return Err(PptxError::InvalidDocument(report));
        }
        let bindings = document
            .source_bindings
            .as_ref()
            .ok_or_else(|| conflict("missing retained binding"))?;
        if bindings.profile != SourceBindingProfile::PresentationmlRetainedFieldsV5 {
            return Err(conflict("source profile does not admit authored pages"));
        }
        let retained_count = bindings.slides.len();
        if document
            .slide_order
            .iter()
            .take(retained_count)
            .any(|id| !bindings.slides.contains_key(id))
            || document
                .slide_order
                .iter()
                .skip(retained_count)
                .any(|id| bindings.slides.contains_key(id))
        {
            return Err(conflict(
                "new pages must follow the unchanged source page order",
            ));
        }
        let mut retained = document.clone();
        retained.slide_order.truncate(retained_count);
        retained
            .slides
            .retain(|id, _| bindings.slides.contains_key(id));
        retained
            .objects
            .retain(|id, _| bindings.objects.contains_key(id));
        retained.resources.retain(|id, _| id == &bindings.resource);
        retained.fonts.clear();
        retained
            .timelines
            .retain(|id, _| bindings.slides.contains_key(id));
        let mut author = Document::empty(document.id.clone(), document.page_size);
        author.title.clone_from(&document.title);
        author.slide_order = document.slide_order[retained_count..].to_vec();
        author.slides = document
            .slides
            .iter()
            .filter(|(id, _)| !bindings.slides.contains_key(*id))
            .map(|(id, s)| (id.clone(), s.clone()))
            .collect();
        author.objects = document
            .objects
            .iter()
            .filter(|(id, _)| !bindings.objects.contains_key(*id))
            .map(|(id, o)| (id.clone(), o.clone()))
            .collect();
        author.resources = document
            .resources
            .iter()
            .filter(|(id, _)| *id != &bindings.resource)
            .map(|(id, r)| (id.clone(), r.clone()))
            .collect();
        author.fonts.clone_from(&document.fonts);
        author.timelines = document
            .timelines
            .iter()
            .filter(|(id, _)| !bindings.slides.contains_key(*id))
            .map(|(id, t)| (id.clone(), t.clone()))
            .collect();

        let input = resources.open(&bindings.resource)?;
        let source = Package::open(input.reader, input.byte_length, package_limits, check)?;
        let retained_plan = SourcePlan::new(
            &retained,
            &source,
            SourceLimits {
                package: package_limits,
                ..Default::default()
            },
            check,
        )?;
        let retained_package = retained_plan.write_to(&source, Vec::new(), check)?;
        let author_plan = AuthorPlan::new(&author, defaults, document_limits, check)?;
        let authored =
            crate::export_plan_to(&author_plan, resources, Vec::new(), package_limits, check)?;
        let appended = graft::append(
            retained_package.package(),
            authored.package(),
            author_plan.bindings(),
            retained_plan.declarations(),
            package_limits,
            check,
        )?;
        let declarations = crate::source::inspect_source(
            appended.package(),
            SourceLimits {
                package: package_limits,
                ..Default::default()
            },
            check,
        )?;
        if declarations.slides.len() != document.slide_order.len() {
            return Err(conflict("composed page count"));
        }
        let mut object_bindings: BTreeMap<_, _> = bindings
            .objects
            .iter()
            .map(|(id, b)| ((b.part.clone(), b.native_id), id.clone()))
            .collect();
        let ids: BTreeMap<_, _> = author_plan
            .bindings()
            .object_ids
            .iter()
            .map(|(id, native)| (*native, id))
            .collect();
        let prefix = graft::prefix(authored.package().sha256());
        for (part, surface) in &author_plan.declarations().surfaces {
            for object in &surface.objects {
                if let Some(id) = ids.get(&object.native_id) {
                    object_bindings
                        .insert((format!("{prefix}{part}"), object.native_id), (*id).clone());
                }
            }
        }
        Ok(Self {
            package: appended,
            declarations,
            object_bindings,
        })
    }
    pub fn identity(&self) -> &Digest {
        self.package.package().sha256()
    }
    pub fn declarations(&self) -> &SourceIndex {
        &self.declarations
    }
    pub fn bytes(&self) -> &[u8] {
        self.package.reader()
    }
    pub fn reader(&self) -> &dyn mo_opc::ReaderAt {
        self.package.reader()
    }
    pub fn object_bindings(&self) -> &BTreeMap<(String, u32), ObjectId> {
        &self.object_bindings
    }
    pub fn write_to<S: ResultSink>(
        &self,
        sink: S,
        check: &dyn Fn() -> bool,
    ) -> Result<VerifiedPackage<S::Reader>, PptxError> {
        Ok(RewritePlan::new().write_sealed(self.package.package(), sink, check)?)
    }
}
fn conflict(message: &str) -> PptxError {
    PptxError::SourceConflict(message.into())
}
