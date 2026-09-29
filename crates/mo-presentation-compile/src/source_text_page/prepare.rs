//! Prepare all visible native frames before a shaping or raster host is called.
//! Table merge origins share one topology/text binding and whole-page budgets.
use super::*;
use crate::source_table::{TableFrameCompiler, TablePreparationBudget};
use mo_presentation_source::source::SourceObject;
use std::collections::BTreeSet;

pub(super) struct PreparationBudget {
    objects: BTreeSet<(String, u32)>,
    indexed_objects: usize,
    frames: usize,
    paragraphs: usize,
    runs: usize,
    plan_bytes: usize,
    tables: TablePreparationBudget,
}
fn charge(remaining: &mut usize, n: usize, label: &'static str) -> Result<(), SourcePageError> {
    *remaining = remaining.checked_sub(n).ok_or(RasterError::Limit(label))?;
    Ok(())
}
impl PreparationBudget {
    pub fn new(limits: TextPageLimits) -> Self {
        Self {
            objects: BTreeSet::new(),
            indexed_objects: limits.tables.max_objects,
            frames: limits.max_frames,
            paragraphs: limits.max_paragraphs,
            runs: limits.max_runs,
            plan_bytes: limits.max_prepared_plan_bytes,
            tables: TablePreparationBudget::new(limits.tables),
        }
    }
    fn frame(
        &mut self,
        paragraphs: &[Vec<mo_presentation_source::source::SourceRun>],
    ) -> Result<(), SourcePageError> {
        charge(&mut self.frames, 1, "page text frames")?;
        charge(&mut self.paragraphs, paragraphs.len(), "page paragraphs")?;
        for paragraph in paragraphs {
            charge(&mut self.runs, paragraph.len(), "page text runs")?;
        }
        Ok(())
    }
}

impl Compiler<'_, '_, '_> {
    #[cfg(test)]
    pub(crate) fn preflight<'b>(
        &mut self,
        index: &SourceIndex,
        q: &SourcePageRequest,
        objects: impl Iterator<Item = &'b SourcePagePaintBinding>,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SourcePageError> {
        self.preflight_using(index, q, objects, None, check)
    }
    pub(crate) fn preflight_shared<'b, 's>(
        &mut self,
        index: &'s SourceIndex,
        q: &SourcePageRequest,
        objects: impl Iterator<Item = &'b SourcePagePaintBinding>,
        tables: &crate::source_table::SharedTables<'s>,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SourcePageError> {
        self.preflight_using(index, q, objects, Some(tables), check)
    }
    fn preflight_using<'b, 's>(
        &mut self,
        index: &'s SourceIndex,
        q: &SourcePageRequest,
        objects: impl Iterator<Item = &'b SourcePagePaintBinding>,
        tables: Option<&crate::source_table::SharedTables<'s>>,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SourcePageError> {
        self.ensure_ready()?;
        self.failed = true;
        let result = self.prepare_objects(index, q, objects, tables, check);
        self.failed = result.is_err();
        result
    }
    fn prepare_objects<'b, 's>(
        &mut self,
        index: &'s SourceIndex,
        q: &SourcePageRequest,
        objects: impl Iterator<Item = &'b SourcePagePaintBinding>,
        tables: Option<&crate::source_table::SharedTables<'s>>,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SourcePageError> {
        if index.source_sha256 != q.expected_source_sha256 {
            return Err(SourcePageError::Invalid("text preparation source digest"));
        }
        let mut indexed = BTreeMap::<&str, BTreeMap<u32, &SourceObject>>::new();
        let mut accounted = BTreeSet::new();
        for object in objects {
            cancel(check)?;
            let at = &object.location;
            let native_id = at
                .object
                .ok_or(SourcePageError::Invalid("text object location"))?;
            let key = (at.part.clone(), native_id);
            if !self.preparation.objects.insert(key.clone()) {
                return Err(SourcePageError::Invalid(
                    "duplicate text object preparation",
                ));
            }
            let surface = index
                .surfaces
                .get(&at.part)
                .ok_or(SourcePageError::Invalid("text object surface"))?;
            if accounted.insert(at.part.as_str()) {
                charge(
                    &mut self.preparation.indexed_objects,
                    surface.objects.len(),
                    "page text source objects",
                )?;
            }
            let shared = tables.and_then(|t| t.get(&key));
            let native = if let Some(table) = shared {
                if !std::ptr::eq(table.source.index(), index)
                    || table.source.part() != at.part
                    || table.source.object().native_id != native_id
                {
                    return Err(SourcePageError::Invalid("shared table source identity"));
                }
                table.source.object()
            } else {
                if let std::collections::btree_map::Entry::Vacant(entry) = indexed.entry(&at.part) {
                    let mut objects = BTreeMap::new();
                    for object in &surface.objects {
                        cancel(check)?;
                        if objects.insert(object.native_id, object).is_some() {
                            return Err(SourcePageError::Invalid("duplicate native text object"));
                        }
                    }
                    entry.insert(objects);
                }
                *indexed[at.part.as_str()]
                    .get(&native_id)
                    .ok_or(SourcePageError::Invalid("missing native text object"))?
            };
            let reference = SourceObjectRef {
                part: at.part.clone(),
                native_id,
            };
            let frames = if let Some(table) = &native.table {
                let limits = self
                    .preparation
                    .tables
                    .admit(table, check)
                    .map_err(SourceFrameError::from)?;
                let table = if let Some(shared) = shared {
                    TableFrameCompiler::bind_prepared(shared, check)
                } else {
                    TableFrameCompiler::bind(
                        index,
                        &q.expected_source_sha256,
                        &reference,
                        limits,
                        check,
                    )
                }
                .map_err(|e| SourcePageError::from(e).at(at))?;
                let mut frames = Vec::new();
                for region in table.geometry().grid().regions() {
                    cancel(check)?;
                    // txBody is optional. Covered cells never contribute extra ink.
                    let Some(body) = table
                        .geometry()
                        .body(region.origin)
                        .map_err(SourceFrameError::from)?
                    else {
                        continue;
                    };
                    self.preparation.frame(body.paragraphs)?;
                    let frame = table
                        .prepare_frame(
                            region.origin,
                            self.manifest,
                            self.limits.work,
                            Fixed::from_raw(1 << 24),
                            check,
                        )
                        .map_err(|e| SourcePageError::from(e).at(at))?;
                    frames.push(self.prepared(index, q, frame, at, check)?);
                }
                frames
            } else if native.text_body_ordinal.is_some() {
                self.preparation.frame(&native.paragraphs)?;
                let frame = source_frame::prepare(
                    index,
                    &SourceFrameRequest {
                        expected_source_sha256: q.expected_source_sha256.clone(),
                        object: reference,
                        bounds_tolerance: Fixed::from_raw(1 << 24),
                    },
                    self.manifest,
                    self.limits.work,
                    check,
                )
                .map_err(|e| SourcePageError::from(e).at(at))?;
                vec![self.prepared(index, q, frame, at, check)?]
            } else {
                Vec::new()
            };
            if !frames.is_empty() {
                self.pending.insert(key, frames);
            }
        }
        cancel(check)
    }
    fn prepared(
        &mut self,
        index: &SourceIndex,
        q: &SourcePageRequest,
        frame: source_frame::PreparedFrame,
        at: &SourcePageLocation,
        check: &dyn Fn() -> bool,
    ) -> Result<Pending, SourcePageError> {
        charge(
            &mut self.preparation.plan_bytes,
            frame.accounted_plan_bytes(),
            "page prepared text plans",
        )?;
        let paints = paint::resolve(
            index,
            frame.source(),
            &q.color_context,
            Default::default(),
            check,
        )
        .map_err(|e| SourcePageError::from(e).at(at))?;
        for (paragraph, runs) in paints.iter().enumerate() {
            for (run, value) in runs.iter().enumerate() {
                glyphs::colors(value).map_err(|e| {
                    SourcePageError::from(e.at_run(paint::TextPaintLocation::at(
                        frame.source().paragraph_start + paragraph as u32,
                        &frame.source().paragraphs[paragraph].runs[run],
                    )))
                    .at(at)
                })?;
            }
        }
        Ok(Pending { frame, paints })
    }
}
