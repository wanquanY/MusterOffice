//! Lazy decode-order header scan with colour runs and seek checkpoints. The
//! immutable container remains the source of sample addresses and descriptions.
#[cfg(test)]
mod tests;
use crate::{SampleDescription, TrackIndex};
use mo_media::{ErrorKind, MediaError, ReadAt};
use mo_media_avc::{AvcInspector, Picture, ReadBudget};
use std::{collections::BTreeMap, mem::size_of};
type Result<T> = std::result::Result<T, MediaError>;
fn invalid(message: &'static str) -> MediaError {
    MediaError::new(ErrorKind::Invalid, message)
}
fn active(check: &dyn Fn() -> bool) -> Result<()> {
    crate::timing::active(check).map_err(crate::timing_error)
}
#[derive(Clone)]
struct Run {
    at: u32,
    description: u32,
    picture: Option<Picture>,
}
struct Checkpoint {
    at: u32,
    state: AvcInspector,
}
struct TrackScan {
    scanned: u32,
    description: u32,
    state: AvcInspector,
    runs: Vec<Run>,
    checkpoints: Vec<Checkpoint>,
}
pub(crate) struct AvcIndex {
    tracks: BTreeMap<u32, TrackScan>,
    read: ReadBudget,
    remaining_cache: usize,
}
impl AvcIndex {
    pub fn new(cache_bytes: usize) -> Self {
        Self {
            tracks: BTreeMap::new(),
            read: ReadBudget::new(32 << 20, 4_000_000).expect("static AVC scan budget"),
            remaining_cache: cache_bytes,
        }
    }
    /// At most count samples per call. A Worker host can yield between batches
    /// and terminate a stalled ReadAt without inventing asynchronous Rust flags.
    pub fn scan(
        &mut self,
        ordinal: u32,
        track: &TrackIndex,
        through: u32,
        count: u16,
        source: &mut dyn ReadAt,
        check: &dyn Fn() -> bool,
    ) -> Result<u32> {
        active(check)?;
        if count == 0 || count > 2048 || through as usize >= track.samples.len() {
            return Err(invalid("AVC scan window"));
        }
        if track.handler != *b"vide" {
            return Err(invalid("AVC video track"));
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = self.tracks.entry(ordinal) {
            let description = track.samples[0].description;
            let state = configuration(track, description)?;
            let cost = snapshot_cost(&state) + size_of::<TrackScan>();
            reserve(&mut self.remaining_cache, cost)?;
            entry.insert(TrackScan {
                scanned: 0,
                description,
                checkpoints: vec![Checkpoint {
                    at: 0,
                    state: state.clone(),
                }],
                state,
                runs: Vec::new(),
            });
        }
        let scan = self.tracks.get_mut(&ordinal).expect("inserted track scan");
        let end = (through + 1).min(scan.scanned.saturating_add(u32::from(count)));
        while scan.scanned < end {
            active(check)?;
            let at = scan.scanned;
            let sample = &track.samples[at as usize];
            let description_changed = sample.description != scan.description;
            let before = if description_changed {
                configuration(track, sample.description)?
            } else {
                scan.state.clone()
            };
            let mut next = before.clone();
            let picture = next.inspect_at(
                source,
                sample.bytes.offset,
                sample.bytes.length,
                &mut self.read,
                check,
            )?;
            let parameters_changed = next.revision() != before.revision();
            let run_changed = scan.runs.last().is_none_or(|last| {
                last.description != sample.description || last.picture != picture
            });
            let cost = if description_changed {
                snapshot_cost(&before)
            } else {
                0
            } + if parameters_changed {
                snapshot_cost(&next)
            } else {
                0
            } + if run_changed { size_of::<Run>() * 2 } else { 0 };
            // No partial run/checkpoint publication on a limit or cancelled read.
            active(check)?;
            reserve(&mut self.remaining_cache, cost)?;
            if description_changed {
                scan.checkpoints.push(Checkpoint { at, state: before });
            }
            if parameters_changed {
                scan.checkpoints.push(Checkpoint {
                    at: at + 1,
                    state: next.clone(),
                });
            }
            if run_changed {
                scan.runs.push(Run {
                    at,
                    description: sample.description,
                    picture,
                });
            }
            scan.description = sample.description;
            scan.state = next;
            scan.scanned += 1;
        }
        Ok(scan.scanned)
    }
    fn track(&self, track: u32, sample: u32) -> Result<&TrackScan> {
        self.tracks
            .get(&track)
            .filter(|scan| sample < scan.scanned)
            .ok_or_else(|| MediaError::new(ErrorKind::State, "AVC sample has not been scanned"))
    }
    pub fn sample(&self, track: u32, sample: u32) -> Result<(u32, Option<&Picture>)> {
        let scan = self.track(track, sample)?;
        let index = scan.runs.partition_point(|run| run.at <= sample);
        let run = &scan.runs[index - 1];
        Ok((run.description, run.picture.as_ref()))
    }
    pub fn bootstrap(&self, track: u32, sample: u32) -> Result<Vec<u8>> {
        let scan = self.track(track, sample)?;
        let index = scan.checkpoints.partition_point(|point| point.at <= sample);
        scan.checkpoints[index - 1].state.bootstrap()
    }
}
fn snapshot_cost(state: &AvcInspector) -> usize {
    // Charge complete parameter bytes even though snapshots share Arc data;
    // include Vec capacity growth and fixed parameter metadata conservatively.
    (size_of::<Checkpoint>() + state.parameter_bytes() + 32768) * 2
}
fn reserve(remaining: &mut usize, cost: usize) -> Result<()> {
    *remaining = remaining
        .checked_sub(cost)
        .ok_or_else(|| MediaError::new(ErrorKind::Limit, "AVC metadata cache"))?;
    Ok(())
}
fn configuration(track: &TrackIndex, description: u32) -> Result<AvcInspector> {
    let entry = track
        .descriptions
        .get(description as usize)
        .ok_or_else(|| invalid("AVC sample description"))?;
    AvcInspector::from_avcc(avcc(entry)?)
}
fn avcc(entry: &SampleDescription) -> Result<&[u8]> {
    if !matches!(&entry.format, b"avc1" | b"avc3") {
        return Err(MediaError::new(ErrorKind::Unsupported, "AVC sample entry"));
    }
    // ISO VisualSampleEntry has 78 payload bytes before its extension boxes.
    if entry.payload.len() < 78 {
        return Err(invalid("visual sample entry"));
    }
    let mut at = 78;
    let mut found = None;
    while at < entry.payload.len() {
        let bytes = &entry.payload[at..];
        if bytes.len() < 8 {
            return Err(invalid("visual extension header"));
        }
        let size = u32::from_be_bytes(bytes[..4].try_into().expect("four bytes"));
        let (size, header) = if size == 1 {
            if bytes.len() < 16 {
                return Err(invalid("visual extension large size"));
            }
            (
                u64::from_be_bytes(bytes[8..16].try_into().expect("eight bytes")),
                16,
            )
        } else if size == 0 {
            (bytes.len() as u64, 8)
        } else {
            (u64::from(size), 8)
        };
        if size < header as u64 || size > bytes.len() as u64 {
            return Err(invalid("visual extension extent"));
        }
        if &bytes[4..8] == b"avcC" {
            if found.is_some() {
                return Err(invalid("duplicate avcC"));
            }
            found = Some(&bytes[header..size as usize]);
        }
        at += size as usize;
    }
    found.ok_or_else(|| invalid("missing avcC"))
}
