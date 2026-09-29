//! Turn object ancestry into isolated contiguous paint intervals. Whole-object
//! opacity is applied once, after all fills, outlines, pictures and glyphs.
use mo_raster::{OpacityGroup, RasterError};
use std::collections::BTreeSet;

pub(crate) struct OpacityScopes<K> {
    active: Vec<(K, usize)>,
    seen: BTreeSet<K>,
    groups: Vec<OpacityGroup>,
}
impl<K: Clone + Ord> OpacityScopes<K> {
    pub fn new() -> Self {
        Self {
            active: vec![],
            seen: BTreeSet::new(),
            groups: vec![],
        }
    }
    pub fn enter(&mut self, chain: &[(K, u16)], draw: u32) -> Result<(), RasterError> {
        if chain.len() > 64 {
            return Err(RasterError::Limit("object opacity depth"));
        }
        let common = self
            .active
            .iter()
            .zip(chain)
            .take_while(|((key, index), (next, opacity))| {
                key == next && self.groups[*index].opacity == *opacity
            })
            .count();
        while self.active.len() > common {
            let (_, index) = self.active.pop().expect("active scope");
            self.groups[index].end_draw = draw;
        }
        for (key, opacity) in &chain[common..] {
            if self.groups.len() == 4096 {
                return Err(RasterError::Limit("object opacity groups"));
            }
            if !self.seen.insert(key.clone()) {
                return Err(RasterError::Invalid("noncontiguous object opacity scope"));
            }
            self.active.push((key.clone(), self.groups.len()));
            self.groups.push(OpacityGroup {
                first_draw: draw,
                end_draw: draw,
                opacity: *opacity,
            });
        }
        Ok(())
    }
    pub fn finish(mut self, draw: u32) -> Vec<OpacityGroup> {
        for (_, index) in self.active {
            self.groups[index].end_draw = draw;
        }
        self.groups.retain(|g| g.first_draw != g.end_draw);
        self.groups
    }
}

pub(crate) fn author(
    document: &mo_presentation_model::Document,
    paints: &[crate::PagePaintSource],
    values: &std::collections::BTreeMap<mo_common::ObjectId, mo_timeline::ExactValue>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<OpacityGroup>, crate::PageError> {
    if values.is_empty() {
        return Ok(vec![]);
    }
    let values = values
        .iter()
        .map(|(id, v)| Ok((id.clone(), crate::sampled_properties::opacity(v)?)))
        .collect::<Result<std::collections::BTreeMap<_, _>, crate::CompileError>>()?;
    let mut scopes = OpacityScopes::new();
    for (i, paint) in paints.iter().enumerate() {
        crate::cancel(check)?;
        let mut chain = vec![];
        let mut current = paint.object.as_ref();
        let mut depth = 0;
        while let Some(id) = current {
            depth += 1;
            if depth > 64 {
                return Err(RasterError::Limit("object opacity ancestry").into());
            }
            if let Some(value) = values.get(id) {
                chain.push((id.clone(), *value));
            }
            current = match &document.objects[id].parent {
                mo_presentation_model::ContainerId::Group(id) => Some(id),
                _ => None,
            };
        }
        chain.reverse();
        scopes.enter(&chain, i as u32)?;
    }
    Ok(scopes.finish(paints.len() as u32))
}
