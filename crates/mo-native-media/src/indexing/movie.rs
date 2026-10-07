use super::*;
use mo_media_isobmff::{
    movie::{MovieClock, MovieTimeline},
    timing::{TimingError, TimingLimits},
};

impl NativeIndex<'_> {
    /// Transfers only bounded track/edit pages. The selected parser worker
    /// retains sample tables; the same portable mapper serves audio and video.
    pub fn movie_timeline(
        &mut self,
        track: u32,
        unedited_media_end: &ExactRatio,
        limits: TimingLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<MovieTimeline, WorkerError> {
        if limits.max_edits > 100_000 || !(32..=4096).contains(&limits.exact_bits) {
            return Err(WorkerError::Invalid("movie timeline limits"));
        }
        let IndexPage::Track { track: clock } =
            self.query(IndexQuery::Track { track }, cancelled)?
        else {
            return Err(WorkerError::Invalid("movie track response"));
        };
        if clock.edits as usize > limits.max_edits {
            return Err(WorkerError::Media(mo_media::MediaError::new(
                mo_media::ErrorKind::Limit,
                "movie edits budget",
            )));
        }
        let mut edits = Vec::new();
        edits
            .try_reserve_exact(clock.edits as usize)
            .map_err(|_| WorkerError::Invalid("movie edits allocation"))?;
        while edits.len() < clock.edits as usize {
            let count = (clock.edits - edits.len() as u32).min(u32::from(MAX_INDEX_ITEMS)) as u16;
            let IndexPage::Edits { total, items } = self.query(
                IndexQuery::Edits {
                    track,
                    start: edits.len() as u32,
                    count,
                },
                cancelled,
            )?
            else {
                return Err(WorkerError::Invalid("movie edits response"));
            };
            if total != clock.edits || items.len() != usize::from(count) {
                return Err(WorkerError::Invalid("changed movie edit table"));
            }
            edits.extend(items);
        }
        MovieTimeline::from_clock(
            MovieClock {
                track_id: clock.id,
                movie_timescale: self.summary.movie_timescale,
                media_timescale: clock.timescale,
                declared_track_duration: clock.declared_track_duration,
                edits: &edits,
            },
            unedited_media_end,
            limits,
            cancelled,
        )
        .map_err(|e| {
            let kind = match e {
                TimingError::Unsupported(_) => mo_media::ErrorKind::Unsupported,
                TimingError::Cancelled => mo_media::ErrorKind::Cancelled,
                TimingError::Budget => mo_media::ErrorKind::Limit,
                TimingError::CallbackPanic => mo_media::ErrorKind::CallbackPanic,
                _ => mo_media::ErrorKind::Invalid,
            };
            WorkerError::Media(mo_media::MediaError::new(kind, e.to_string()))
        })
    }
}
