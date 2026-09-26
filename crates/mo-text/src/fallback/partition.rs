use crate::{cascade::VariationIssue, *};
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct Partition {
    pub points: Vec<u32>,
    pub allowed: Vec<bool>,
}
impl Partition {
    pub fn new(start: u32, end: u32, boundaries: &BTreeSet<u32>) -> Self {
        let points: Vec<_> = boundaries.range(start..=end).copied().collect();
        let allowed = vec![true; points.len()];
        Self { points, allowed }
    }
    pub fn span(&self, start: u32, end: u32) -> (usize, usize) {
        let lo = self
            .points
            .partition_point(|&p| p <= start)
            .saturating_sub(1);
        let hi = self.points.partition_point(|&p| p < end);
        (lo, hi)
    }
    pub fn observe(
        &mut self,
        run: &ShapedRun,
        issues: &[VariationIssue],
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<u32>, TextError> {
        let mut clusters = BTreeMap::<u32, (bool, bool)>::new();
        for glyph in &run.glyphs {
            cancelled(check)?;
            let flags = clusters.entry(glyph.cluster).or_default();
            flags.0 |= glyph.glyph_id == 0;
            flags.1 |= glyph.unsafe_to_break;
        }
        let mut bad = vec![false; self.points.len() - 1];
        let starts: Vec<_> = clusters
            .keys()
            .copied()
            .chain(std::iter::once(run.end))
            .collect();
        for (pair, (_, &(missing, unsafe_break))) in starts.windows(2).zip(&clusters) {
            cancelled(check)?;
            let (lo, hi) = self.span(pair[0], pair[1]);
            // A glyph cluster can start inside an extended grapheme; both
            // boundaries must survive both coordinate systems before splitting.
            self.allowed[lo + 1..hi].fill(false);
            if unsafe_break
                && let Ok(at) = self.points.binary_search(&pair[0])
                && at > 0
                && at + 1 < self.points.len()
            {
                self.allowed[at] = false;
            }
            if missing {
                bad[lo..hi].fill(true);
            }
        }
        for issue in issues {
            cancelled(check)?;
            let (lo, hi) = self.span(
                issue.base_offset.unwrap_or(issue.selector_offset),
                issue.selector_offset + 1,
            );
            bad[lo..hi].fill(true);
        }
        let mut prefix = Vec::with_capacity(bad.len() + 1);
        prefix.push(0);
        for b in bad {
            cancelled(check)?;
            prefix.push(prefix.last().unwrap() + u32::from(b));
        }
        Ok(prefix)
    }
    pub fn regions(&self) -> Vec<(usize, usize)> {
        let active: Vec<_> = self
            .allowed
            .iter()
            .enumerate()
            .filter_map(|(i, &ok)| ok.then_some(i))
            .collect();
        active.windows(2).map(|w| (w[0], w[1])).collect()
    }
    pub fn protected(&self) -> Vec<u32> {
        self.points
            .iter()
            .zip(&self.allowed)
            .filter_map(|(&p, &ok)| (!ok).then_some(p))
            .collect()
    }
}
