//! Bounded ISO BMFF sample tables over explicit immutable byte capabilities.
//! Keeps decode/composition/movie clocks separate. Does not decode, repair
//! timestamps, fetch external data references, or infer continuous coverage.
mod boxes;
mod tables;
mod track;
use mo_media::ReadAt;
use serde::{Deserialize, Serialize};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub const PROFILE: &str = "musteroffice.isobmff-index/1-draft";
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexLimits {
    pub max_input_bytes: u64,
    pub max_metadata_bytes: usize,
    pub max_boxes: usize,
    pub max_tracks: usize,
    /// Across all tracks, not a renewable per-table limit.
    pub max_samples: usize,
    pub max_edits: usize,
    pub max_sample_descriptions: usize,
    pub max_description_bytes: usize,
}
impl Default for IndexLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 1 << 40,
            max_metadata_bytes: 32 << 20,
            max_boxes: 8192,
            max_tracks: 64,
            max_samples: 2_000_000,
            max_edits: 4096,
            max_sample_descriptions: 128,
            max_description_bytes: 1 << 20,
        }
    }
}
impl IndexLimits {
    pub fn validate(&self) -> Result<()> {
        if self.max_input_bytes == 0
            || self.max_input_bytes > i64::MAX as u64
            || self.max_metadata_bytes == 0
            || self.max_metadata_bytes > 128 << 20
            || self.max_boxes == 0
            || self.max_boxes > 100_000
            || self.max_tracks == 0
            || self.max_tracks > 1024
            || self.max_samples == 0
            || self.max_samples > 10_000_000
            || self.max_edits > 100_000
            || self.max_sample_descriptions > 4096
            || self.max_description_bytes > 16 << 20
        {
            return Err(IndexError::Invalid("index limits"));
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("ISO BMFF I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid ISO BMFF: {0}")]
    Invalid(&'static str),
    #[error("unsupported ISO BMFF feature: {0}")]
    Unsupported(&'static str),
    #[error("ISO BMFF index budget exceeded")]
    Budget,
    #[error("ISO BMFF indexing cancelled")]
    Cancelled,
    #[error("ISO BMFF callback panicked")]
    CallbackPanic,
}
pub type Result<T> = std::result::Result<T, IndexError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ByteRange {
    pub offset: u64,
    pub length: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleDescription {
    pub format: [u8; 4],
    /// Original sample-entry bytes after its box header. Includes the checked
    /// data-reference index and codec initialization. Never executable code.
    pub payload: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sample {
    pub bytes: ByteRange,
    /// Raw track-media ticks before edit lists, in decode order.
    pub dts: i64,
    pub pts: i64,
    pub decode_duration: u32,
    /// Zero based index into this track's sample descriptions.
    pub description: u32,
    /// stss membership (or all when absent), not an unconditional closed-GOP
    /// promise. Codec leading pictures and dependency groups remain relevant.
    pub sync_sample: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Edit {
    /// Consecutive movie-clock ticks. No rescaling to the track timebase.
    pub movie_start: u64,
    pub movie_duration: u64,
    /// None means an explicit empty edit; otherwise raw track-media ticks.
    pub media_time: Option<i64>,
    /// Exact signed 16.16 rate. Zero denotes dwell; no conversion to float.
    pub rate_16_16: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleAuxiliary {
    pub kind: [u8; 4],
    pub payload: Vec<u8>,
}
#[derive(Debug)]
pub struct TrackIndex {
    pub id: u32,
    pub flags: u32,
    pub handler: [u8; 4],
    /// Track-header duration is in movie ticks, distinct from mdhd/stts clocks.
    pub declared_track_duration: Option<u64>,
    pub timescale: u32,
    pub declared_media_duration: Option<u64>,
    pub decode_duration: u64,
    pub display_matrix: [i32; 9],
    pub display_size_16_16: [u32; 2],
    pub descriptions: Vec<SampleDescription>,
    pub samples: Vec<Sample>,
    /// Empty means no edits were declared. Never synthesize one from duration.
    pub edits: Vec<Edit>,
    /// Dependency/roll/recovery metadata preserved for decoder admission.
    pub auxiliary: Vec<SampleAuxiliary>,
}
#[derive(Debug)]
pub struct IsoBmffIndex {
    pub movie_timescale: u32,
    pub declared_movie_duration: Option<u64>,
    pub tracks: Vec<TrackIndex>,
    pub media_data: Vec<ByteRange>,
    pub bytes_read: u64,
}

impl IsoBmffIndex {
    /// Read only top-level headers and bounded movie metadata. `length` and
    /// content identity are supplied/verified by the authorized host. Unknown
    /// top-level boxes are skipped by checked length, never buffered as media.
    pub fn read(
        source: &mut dyn ReadAt,
        length: u64,
        limits: IndexLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self> {
        match catch_unwind(AssertUnwindSafe(|| read(source, length, limits, cancelled))) {
            Ok(result) => result,
            Err(payload) => {
                std::mem::forget(payload);
                Err(IndexError::CallbackPanic)
            }
        }
    }
}
pub(crate) struct Budget<'a> {
    pub limits: IndexLimits,
    pub boxes: usize,
    pub samples: usize,
    pub edits: usize,
    pub descriptions: usize,
    pub check: &'a dyn Fn() -> bool,
}
impl Budget<'_> {
    pub fn active(&self) -> Result<()> {
        if (self.check)() {
            Err(IndexError::Cancelled)
        } else {
            Ok(())
        }
    }
    pub fn box_read(&mut self) -> Result<()> {
        self.active()?;
        if self.boxes >= self.limits.max_boxes {
            return Err(IndexError::Budget);
        }
        self.boxes += 1;
        Ok(())
    }
}
pub(crate) fn reserve<T>(vector: &mut Vec<T>, count: usize) -> Result<()> {
    // Table-sized allocations are exact. Incrementally discovered boxes use
    // amortized growth, avoiding a quadratic reallocation sequence.
    let result = if count == 1 {
        vector.try_reserve(count)
    } else {
        vector.try_reserve_exact(count)
    };
    result.map_err(|_| IndexError::Budget)
}

fn read(
    source: &mut dyn ReadAt,
    length: u64,
    limits: IndexLimits,
    check: &dyn Fn() -> bool,
) -> Result<IsoBmffIndex> {
    limits.validate()?;
    if length < 8 {
        return Err(IndexError::Invalid("file header"));
    }
    if length > limits.max_input_bytes {
        return Err(IndexError::Budget);
    }
    let mut budget = Budget {
        limits,
        boxes: 0,
        samples: 0,
        edits: 0,
        descriptions: 0,
        check,
    };
    let (movie, media_data, bytes_read) = boxes::read_movie(source, length, &mut budget)?;
    let children = boxes::children(&movie, &mut budget)?;
    if children
        .iter()
        .any(|b| matches!(&b.kind, b"mvex" | b"cmov"))
    {
        return Err(IndexError::Unsupported("fragmented or compressed movie"));
    }
    let header = boxes::required(&children, b"mvhd")?;
    let (movie_timescale, declared_movie_duration) = track::clock(header)?;
    let mut tracks = Vec::new();
    for child in children.iter().filter(|b| b.kind == *b"trak") {
        budget.active()?;
        if tracks.len() >= limits.max_tracks {
            return Err(IndexError::Budget);
        }
        let index = track::read_track(child.data, &media_data, &mut budget)?;
        if tracks.iter().any(|t: &TrackIndex| t.id == index.id) {
            return Err(IndexError::Invalid("duplicate track ID"));
        }
        reserve(&mut tracks, 1)?;
        tracks.push(index);
    }
    if tracks.is_empty() {
        return Err(IndexError::Invalid("movie has no tracks"));
    }
    budget.active()?;
    Ok(IsoBmffIndex {
        movie_timescale,
        declared_movie_duration,
        tracks,
        media_data,
        bytes_read,
    })
}
