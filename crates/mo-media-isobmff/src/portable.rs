//! Portable JSON boundary. All 64-bit integers are decimal strings; bounded
//! u32/u16 values remain numbers. Native private transport keeps its own encoding.
mod values;
use crate::{IndexLimits, IndexedMovie, movie::MovieTimeline, query::*, timing::TimingLimits};
use mo_media::{ErrorKind, MediaError, ReadAt};
use mo_time::{ExactRatio, SampleSide};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
pub const PROFILE: &str = "musteroffice.media-index/1-draft";
pub type Result<T> = std::result::Result<T, MediaError>;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selection {
    track: u32,
    time: ExactRatio,
    side: SampleSide,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MovieRequest {
    track: u32,
    unedited_media_end: ExactRatio,
    start: u32,
    count: u16,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AvcScanRequest {
    track: u32,
    through: u32,
    count: u16,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AvcSampleRequest {
    track: u32,
    sample: u32,
}
pub struct PortableIndex {
    inner: IndexedMovie,
    limits: IndexLimits,
    avc: crate::avc::AvcIndex,
}
pub fn decode<T: DeserializeOwned>(input: &str) -> Result<T> {
    if input.len() > 16384 {
        return Err(MediaError::new(
            ErrorKind::Limit,
            "index request byte limit",
        ));
    }
    mo_common::from_json_str_with_path(input)
        .map_err(|_| MediaError::new(ErrorKind::Invalid, "invalid index request"))
}
pub fn encode(value: Value) -> Result<String> {
    let text = serde_json::to_string(&value)
        .map_err(|_| MediaError::new(ErrorKind::State, "index serialization"))?;
    if text.len() > 2 << 20 {
        return Err(MediaError::new(
            ErrorKind::Limit,
            "index response byte limit",
        ));
    }
    Ok(text)
}
impl PortableIndex {
    pub fn open(
        source: &mut dyn ReadAt,
        length: u64,
        limits: IndexLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        Ok(Self {
            inner: IndexedMovie::open(source, length, limits, check)?,
            limits,
            avc: crate::avc::AvcIndex::new(limits.max_metadata_bytes),
        })
    }
    pub fn summary(&self) -> Result<String> {
        encode(values::summary(IndexSummary::from_index(self.inner.raw())))
    }
    /// A separate bounded lazy cache. ReadAt must address the same immutable
    /// bytes supplied to open. Scan in batches, then query without further I/O.
    pub fn scan_avc(
        &mut self,
        request: &str,
        source: &mut dyn ReadAt,
        check: &dyn Fn() -> bool,
    ) -> Result<String> {
        let request: AvcScanRequest = decode(request)?;
        let track = self
            .inner
            .raw()
            .tracks
            .get(request.track as usize)
            .ok_or_else(|| MediaError::new(ErrorKind::Invalid, "AVC track index"))?;
        let scanned = self.avc.scan(
            request.track,
            track,
            request.through,
            request.count,
            source,
            check,
        )?;
        encode(json!({"scanned":scanned,"ready":scanned > request.through}))
    }
    pub fn avc_sample(&self, request: &str) -> Result<String> {
        let request: AvcSampleRequest = decode(request)?;
        let (description, picture) = self.avc.sample(request.track, request.sample)?;
        // Preserve field metadata even when the current progressive renderer
        // cannot interpret it. No unsupported field location becomes a guess.
        let location = picture
            .and_then(|p| p.sequence.progressive_chroma_location().ok())
            .flatten();
        encode(
            json!({"sample":request.sample,"description":description,"picture":picture,"chromaLocation":location}),
        )
    }
    pub fn avc_bootstrap(&self, request: &str) -> Result<Vec<u8>> {
        let request: AvcSampleRequest = decode(request)?;
        self.avc.bootstrap(request.track, request.sample)
    }
    pub fn query(&self, request: &str) -> Result<String> {
        let query: IndexQuery = decode(request)?;
        let page = query
            .execute(self.inner.raw())
            .map_err(|_| MediaError::new(ErrorKind::Invalid, "index query"))?;
        encode(values::page(page))
    }
    pub fn video_timing(&mut self, track: u32, check: &dyn Fn() -> bool) -> Result<String> {
        encode(values::video_summary(
            self.inner.timing(track, check)?.summary(),
        ))
    }
    pub fn select_video(&mut self, request: &str, check: &dyn Fn() -> bool) -> Result<String> {
        let request: Selection = decode(request)?;
        let selection = self
            .inner
            .timing(request.track, check)?
            .select(&request.time, request.side, check)
            .map_err(crate::timing_error)?;
        encode(values::selection(selection))
    }
    /// Audio and video share exact container edits. Caller supplies the decoded
    /// raw endpoint only for the no-edit/no-declared-duration fallback case.
    pub fn movie(&self, request: &str, check: &dyn Fn() -> bool) -> Result<String> {
        let request: MovieRequest = decode(request)?;
        let raw = self.inner.raw();
        let track = raw
            .tracks
            .get(request.track as usize)
            .ok_or_else(|| MediaError::new(ErrorKind::Invalid, "track index"))?;
        let movie = MovieTimeline::build(
            track,
            raw.movie_timescale,
            &request.unedited_media_end,
            TimingLimits {
                max_samples: self.limits.max_samples,
                max_edits: self.limits.max_edits,
                exact_bits: 256,
            },
            check,
        )
        .map_err(crate::timing_error)?;
        let start = request.start as usize;
        if request.count == 0 || request.count > MAX_INDEX_ITEMS || start > movie.segments().len() {
            return Err(MediaError::new(ErrorKind::Invalid, "movie page window"));
        }
        let end = movie
            .segments()
            .len()
            .min(start + usize::from(request.count));
        encode(
            json!({"trackId": movie.track_id(), "movieTimescale": movie.movie_timescale(),
            "mediaTimescale": movie.media_timescale(), "end": movie.end(), "total":movie.segments().len(), "start":request.start,
            "segments": movie.segments()[start..end].iter().map(|s| json!({"movie": s.movie(),
                "mediaOrigin": s.media_origin(), "rate": s.rate(), "editIndex": s.edit_index()})).collect::<Vec<_>>() }),
        )
    }
}
