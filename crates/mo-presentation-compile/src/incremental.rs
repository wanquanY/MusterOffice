//! Bounded, host-owned static page plans. A reuse key covers the model's visual
//! dependency closure and every compiler option. Untrusted ChangeSets never
//! authorize reuse; model admission and cancellation also apply to cache hits.
use crate::{CompileError, PageError, PagePlan, PageRenderRequest};
use mo_common::{Digest, DocumentId, SlideId};
use mo_presentation_model::{PageDependencies, ValidationLimits};
use std::collections::VecDeque;

pub struct PagePlanCache {
    max_pages: usize,
    max_commands: u64,
    commands: u64,
    entries: VecDeque<Entry>,
}
struct Entry {
    document: DocumentId,
    slide: SlideId,
    key: Digest,
    plan: PagePlan,
}
pub struct PageCompilation {
    pub plan: PagePlan,
    pub reused: bool,
}
impl PagePlanCache {
    /// Budgets bound retained page count and generated path commands. This is
    /// a logical work budget, not a claim about allocator overhead or RSS.
    /// Zero budgets disable retention without changing compilation semantics.
    pub fn new(max_pages: usize, max_commands: u64) -> Self {
        Self {
            max_pages,
            max_commands,
            commands: 0,
            entries: VecDeque::new(),
        }
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.commands = 0;
    }
    pub fn compile(
        &mut self,
        request: &PageRenderRequest,
        check: &dyn Fn() -> bool,
    ) -> Result<PageCompilation, PageError> {
        crate::cancel(check)?;
        let document = &request.page.document;
        let report = mo_presentation_model::validate(
            document,
            ValidationLimits {
                max_objects: 8192,
                ..Default::default()
            },
        );
        if !report.is_valid() {
            return Err(CompileError::Document(report).into());
        }
        let dependencies = PageDependencies::new(document, &request.page.slide)
            .ok_or(CompileError::Invalid("page dependency reference"))?;
        let key = mo_common::digest(
            "musteroffice.presentation.page-plan/1",
            &(
                crate::PAGE_PROFILE,
                crate::PROFILE,
                mo_render::PROFILE,
                dependencies,
                &request.viewport,
                &request.defaults,
            ),
        )
        .map_err(|_| CompileError::Invalid("page plan identity"))?;
        crate::cancel(check)?;
        if let Some(at) = self.entries.iter().position(|entry| {
            entry.document == document.id && entry.slide == request.page.slide && entry.key == key
        }) {
            let document_sha256 = document
                .semantic_digest()
                .map_err(|_| CompileError::Invalid("document identity"))?;
            crate::cancel(check)?;
            let mut entry = self.entries.remove(at).expect("located cache entry");
            // Provenance follows the current immutable document even when its
            // visual projection is unchanged (title, accessibility, timeline).
            entry.plan.info.document_sha256 = document_sha256;
            let plan = entry.plan.clone();
            self.entries.push_back(entry);
            return Ok(PageCompilation { plan, reused: true });
        }
        let plan = crate::compile_page(request, check)?;
        crate::cancel(check)?;
        let commands = u64::from(plan.info.generated_commands);
        if self.max_pages > 0 && self.max_commands > 0 && commands <= self.max_commands {
            self.entries.retain(|entry| {
                if entry.document == document.id && entry.slide == request.page.slide {
                    self.commands -= u64::from(entry.plan.info.generated_commands);
                    false
                } else {
                    true
                }
            });
            while self.entries.len() >= self.max_pages
                || self.commands > self.max_commands - commands
            {
                let entry = self.entries.pop_front().expect("budgeted cache entry");
                self.commands -= u64::from(entry.plan.info.generated_commands);
            }
            self.entries.push_back(Entry {
                document: document.id.clone(),
                slide: request.page.slide.clone(),
                key,
                plan: plan.clone(),
            });
            self.commands += commands;
        }
        Ok(PageCompilation {
            plan,
            reused: false,
        })
    }
}
