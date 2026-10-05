use crate::{CompileError, Result, Work};
use mo_common::RationalTime;
use mo_time::ExactRatio;
use mo_video_model::{SampleSide, TimeRange, TimeSample};

/// Immutable balanced interval index with subtree max-end pruning, including overlapping audio.
/// Entries refer to the original track order, independent of sorted timeline positions.
#[derive(Debug)]
pub(crate) struct IntervalIndex {
    nodes: Vec<Node>,
    root: Option<usize>,
}
#[derive(Debug)]
struct Node {
    order: usize,
    range: TimeRange,
    max_end: RationalTime,
    left: Option<usize>,
    right: Option<usize>,
}
impl IntervalIndex {
    pub fn new(mut entries: Vec<(usize, TimeRange)>, check: &dyn Fn() -> bool) -> Result<Self> {
        if check() {
            return Err(CompileError::Cancelled);
        }
        entries.sort_unstable_by(|(a, ar), (b, br)| ar.start.compare_time(br.start).then(a.cmp(b)));
        let mut index = Self {
            nodes: Vec::with_capacity(entries.len()),
            root: None,
        };
        index.root = index.build(&entries, check)?;
        Ok(index)
    }
    fn build(
        &mut self,
        entries: &[(usize, TimeRange)],
        check: &dyn Fn() -> bool,
    ) -> Result<Option<usize>> {
        if check() {
            return Err(CompileError::Cancelled);
        }
        if entries.is_empty() {
            return Ok(None);
        }
        let middle = entries.len() / 2;
        let (order, range) = entries[middle];
        let left = self.build(&entries[..middle], check)?;
        let right = self.build(&entries[middle + 1..], check)?;
        let mut max_end = range.end;
        for id in [left, right].into_iter().flatten() {
            if self.nodes[id].max_end.compare_time(max_end).is_gt() {
                max_end = self.nodes[id].max_end;
            }
        }
        let id = self.nodes.len();
        self.nodes.push(Node {
            order,
            range,
            max_end,
            left,
            right,
        });
        Ok(Some(id))
    }
    pub fn at(&self, sample: &TimeSample, work: &mut Work<'_>) -> Result<Vec<usize>> {
        let mut hits = vec![];
        self.query(self.root, sample, work, &mut hits)?;
        hits.sort_unstable();
        Ok(hits)
    }
    fn query(
        &self,
        id: Option<usize>,
        sample: &TimeSample,
        work: &mut Work<'_>,
        hits: &mut Vec<usize>,
    ) -> Result<()> {
        let Some(id) = id else {
            return Ok(());
        };
        work.visit()?;
        let node = &self.nodes[id];
        let end = ExactRatio::from_time(node.max_end).compare(&sample.time);
        if match sample.side {
            SampleSide::After => end.is_le(),
            SampleSide::Before => end.is_lt(),
        } {
            return Ok(());
        }
        self.query(node.left, sample, work, hits)?;
        let start = ExactRatio::from_time(node.range.start).compare(&sample.time);
        if match sample.side {
            SampleSide::After => start.is_gt(),
            SampleSide::Before => start.is_ge(),
        } {
            return Ok(());
        }
        if sample.covers(node.range) {
            if hits.len() >= work.limits.max_nodes {
                return Err(CompileError::Budget);
            }
            hits.push(node.order);
        }
        self.query(node.right, sample, work, hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QueryLimits;
    use proptest::prelude::*;

    fn range(start: i64, end: i64) -> TimeRange {
        TimeRange {
            start: RationalTime::new(start, 1).unwrap(),
            end: RationalTime::new(end, 1).unwrap(),
        }
    }
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2048))]
        #[test]
        fn indexed_lookup_matches_independent_scan_with_gaps_overlaps_and_boundaries(
            intervals in prop::collection::vec((-1000i64..1000,1i64..100), 0..200), tick in -1100i64..1200,
            before in any::<bool>()) {
            let entries: Vec<_> = intervals.iter().enumerate().map(|(n,&(s,d))|(n,range(s,s+d))).collect();
            let sample = TimeSample { time:ExactRatio::integer(tick),side:if before {SampleSide::Before}else{SampleSide::After} };
            let expected: Vec<_> = entries.iter().filter(|(_,r)| if before {
                r.start.ticks.get()<tick && tick<=r.end.ticks.get()
            }else{r.start.ticks.get()<=tick && tick<r.end.ticks.get()}).map(|(n,_)|*n).collect();
            let index = IntervalIndex::new(entries,&||false).unwrap();
            let mut work=Work::new(QueryLimits::default(),&||false).unwrap();
            prop_assert_eq!(index.at(&sample,&mut work).unwrap(),expected);
        }
    }
    #[test]
    fn sparse_hundred_thousand_clip_query_does_not_scan_the_track() {
        let index = IntervalIndex::new(
            (0..100_000)
                .map(|n| (n, range(n as i64 * 2, n as i64 * 2 + 1)))
                .collect(),
            &|| false,
        )
        .unwrap();
        for tick in [0, 99999, 199998, 200000] {
            let mut work = Work::new(QueryLimits::default(), &|| false).unwrap();
            let hits = index
                .at(
                    &TimeSample::after(RationalTime::new(tick, 1).unwrap()),
                    &mut work,
                )
                .unwrap();
            assert!(work.stats.visited < 64, "{} visits", work.stats.visited);
            assert_eq!(hits.len(), usize::from(tick % 2 == 0 && tick < 200000));
        }
    }
}
