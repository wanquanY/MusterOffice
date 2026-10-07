//! Bounded views of an immutable raw container index. No implicit whole-index transfer.
use crate::{ByteRange, Edit, IndexLimits, IsoBmffIndex, Sample, TrackIndex};
use serde::{Deserialize, Serialize};
use std::io;

pub const MAX_INDEX_ITEMS: u16 = 128;
pub const MAX_INDEX_BLOB_BYTES: u16 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum IndexQuery {
    Track {
        track: u32,
    },
    Samples {
        track: u32,
        start: u32,
        count: u16,
    },
    Edits {
        track: u32,
        start: u32,
        count: u16,
    },
    MediaData {
        start: u32,
        count: u16,
    },
    Description {
        track: u32,
        index: u32,
        offset: u32,
        length: u16,
    },
    Auxiliary {
        track: u32,
        index: u32,
        offset: u32,
        length: u16,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexSummary {
    pub movie_timescale: u32,
    pub declared_movie_duration: Option<u64>,
    pub tracks: u32,
    pub media_data: u32,
    pub bytes_read: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexTrack {
    pub id: u32,
    pub flags: u32,
    pub handler: [u8; 4],
    pub declared_track_duration: Option<u64>,
    pub timescale: u32,
    pub declared_media_duration: Option<u64>,
    pub decode_duration: u64,
    pub display_matrix: [i32; 9],
    pub display_size_16_16: [u32; 2],
    pub descriptions: u32,
    pub samples: u32,
    pub edits: u32,
    pub auxiliary: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum IndexPage {
    Track {
        track: IndexTrack,
    },
    Samples {
        total: u32,
        items: Vec<Sample>,
    },
    Edits {
        total: u32,
        items: Vec<Edit>,
    },
    MediaData {
        total: u32,
        items: Vec<ByteRange>,
    },
    Blob {
        kind: [u8; 4],
        total: u32,
        bytes: Vec<u8>,
    },
}
fn invalid() -> io::Error {
    io::Error::other("invalid container index query or response")
}
fn window(total: usize, start: u32, count: u16, cap: u16) -> io::Result<std::ops::Range<usize>> {
    let start = start as usize;
    if count == 0 || count > cap || start > total {
        return Err(invalid());
    }
    Ok(start..total.min(start + usize::from(count)))
}
impl IndexSummary {
    pub fn from_index(index: &IsoBmffIndex) -> Self {
        Self {
            movie_timescale: index.movie_timescale,
            declared_movie_duration: index.declared_movie_duration,
            tracks: index.tracks.len() as u32,
            media_data: index.media_data.len() as u32,
            bytes_read: index.bytes_read,
        }
    }
    pub fn validate(&self, limits: IndexLimits, max_read_bytes: u64) -> io::Result<()> {
        if self.movie_timescale == 0
            || self.tracks == 0
            || self.tracks as usize > limits.max_tracks
            || self.media_data as usize > limits.max_boxes
            || self.bytes_read > max_read_bytes
        {
            return Err(invalid());
        }
        Ok(())
    }
}
impl From<&TrackIndex> for IndexTrack {
    fn from(track: &TrackIndex) -> Self {
        Self {
            id: track.id,
            flags: track.flags,
            handler: track.handler,
            declared_track_duration: track.declared_track_duration,
            timescale: track.timescale,
            declared_media_duration: track.declared_media_duration,
            decode_duration: track.decode_duration,
            display_matrix: track.display_matrix,
            display_size_16_16: track.display_size_16_16,
            descriptions: track.descriptions.len() as u32,
            samples: track.samples.len() as u32,
            edits: track.edits.len() as u32,
            auxiliary: track.auxiliary.len() as u32,
        }
    }
}
impl IndexQuery {
    pub fn validate(self, summary: IndexSummary) -> io::Result<()> {
        let track = match self {
            Self::Track { track } => Some(track),
            Self::Samples { track, count, .. } | Self::Edits { track, count, .. } => {
                window(0, 0, count, MAX_INDEX_ITEMS)?;
                Some(track)
            }
            Self::MediaData { start, count } => {
                window(summary.media_data as usize, start, count, MAX_INDEX_ITEMS)?;
                None
            }
            Self::Description { track, length, .. } | Self::Auxiliary { track, length, .. } => {
                window(0, 0, length, MAX_INDEX_BLOB_BYTES)?;
                Some(track)
            }
        };
        if track.is_some_and(|t| t >= summary.tracks) {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn execute(self, index: &IsoBmffIndex) -> io::Result<IndexPage> {
        self.validate(IndexSummary::from_index(index))?;
        let get = |track: u32| index.tracks.get(track as usize).ok_or_else(invalid);
        Ok(match self {
            Self::Track { track } => IndexPage::Track {
                track: get(track)?.into(),
            },
            Self::Samples {
                track,
                start,
                count,
            } => {
                let all = &get(track)?.samples;
                IndexPage::Samples {
                    total: all.len() as u32,
                    items: all[window(all.len(), start, count, MAX_INDEX_ITEMS)?].to_vec(),
                }
            }
            Self::Edits {
                track,
                start,
                count,
            } => {
                let all = &get(track)?.edits;
                IndexPage::Edits {
                    total: all.len() as u32,
                    items: all[window(all.len(), start, count, MAX_INDEX_ITEMS)?].to_vec(),
                }
            }
            Self::MediaData { start, count } => {
                let all = &index.media_data;
                IndexPage::MediaData {
                    total: all.len() as u32,
                    items: all[window(all.len(), start, count, MAX_INDEX_ITEMS)?].to_vec(),
                }
            }
            Self::Description {
                track,
                index,
                offset,
                length,
            } => {
                let entry = get(track)?
                    .descriptions
                    .get(index as usize)
                    .ok_or_else(invalid)?;
                blob(entry.format, &entry.payload, offset, length)?
            }
            Self::Auxiliary {
                track,
                index,
                offset,
                length,
            } => {
                let entry = get(track)?
                    .auxiliary
                    .get(index as usize)
                    .ok_or_else(invalid)?;
                blob(entry.kind, &entry.payload, offset, length)?
            }
        })
    }
}
fn blob(kind: [u8; 4], bytes: &[u8], offset: u32, length: u16) -> io::Result<IndexPage> {
    Ok(IndexPage::Blob {
        kind,
        total: bytes.len() as u32,
        bytes: bytes[window(bytes.len(), offset, length, MAX_INDEX_BLOB_BYTES)?].to_vec(),
    })
}
impl IndexPage {
    pub fn validate(
        &self,
        query: IndexQuery,
        summary: IndexSummary,
        limits: IndexLimits,
        file_length: u64,
    ) -> io::Result<()> {
        query.validate(summary)?;
        let valid_range = |range: &ByteRange| {
            range.offset <= file_length && range.length <= file_length - range.offset
        };
        let check = |total: u32,
                     len: usize,
                     start: u32,
                     count: u16,
                     cap: u16,
                     max: usize|
         -> io::Result<()> {
            if total as usize > max || window(total as usize, start, count, cap)?.len() != len {
                return Err(invalid());
            }
            Ok(())
        };
        match (self, query) {
            (Self::Track { track }, IndexQuery::Track { .. })
                if track.id > 0
                    && track.timescale > 0
                    && track.samples as usize <= limits.max_samples
                    && track.edits as usize <= limits.max_edits
                    && track.descriptions > 0
                    && track.descriptions as usize <= limits.max_sample_descriptions
                    && track.auxiliary as usize <= limits.max_boxes =>
            {
                Ok(())
            }
            (Self::Samples { total, items }, IndexQuery::Samples { start, count, .. }) => {
                check(
                    *total,
                    items.len(),
                    start,
                    count,
                    MAX_INDEX_ITEMS,
                    limits.max_samples,
                )?;
                if items.iter().any(|s| {
                    !valid_range(&s.bytes)
                        || s.bytes.length == 0
                        || s.decode_duration == 0
                        || s.description as usize >= limits.max_sample_descriptions
                }) {
                    return Err(invalid());
                }
                Ok(())
            }
            (Self::Edits { total, items }, IndexQuery::Edits { start, count, .. }) => {
                check(
                    *total,
                    items.len(),
                    start,
                    count,
                    MAX_INDEX_ITEMS,
                    limits.max_edits,
                )?;
                if items.iter().any(|e| {
                    e.movie_start.checked_add(e.movie_duration).is_none()
                        || e.media_time.is_some_and(|t| t < 0)
                }) {
                    return Err(invalid());
                }
                Ok(())
            }
            (Self::MediaData { total, items }, IndexQuery::MediaData { start, count }) => {
                check(
                    *total,
                    items.len(),
                    start,
                    count,
                    MAX_INDEX_ITEMS,
                    limits.max_boxes,
                )?;
                if *total != summary.media_data || items.iter().any(|r| !valid_range(r)) {
                    return Err(invalid());
                }
                Ok(())
            }
            (Self::Blob { total, bytes, .. }, IndexQuery::Description { offset, length, .. }) => {
                check(
                    *total,
                    bytes.len(),
                    offset,
                    length,
                    MAX_INDEX_BLOB_BYTES,
                    limits.max_description_bytes,
                )
            }
            (Self::Blob { total, bytes, .. }, IndexQuery::Auxiliary { offset, length, .. }) => {
                check(
                    *total,
                    bytes.len(),
                    offset,
                    length,
                    MAX_INDEX_BLOB_BYTES,
                    limits.max_metadata_bytes,
                )
            }
            _ => Err(invalid()),
        }
    }
}
