//! Real decoder → provisional local files → sealed canonical reader. This is a
//! test I/O capability, deliberately not product persistence or authorization.
use super::*;
use mo_video_runtime::authoring::model::StreamId;
use mo_video_runtime::common::{Digest, ResourceId, digest};
use std::{collections::BTreeMap, io::Write, path::PathBuf, sync::Arc};
type Manifest = (PcmStreamRecord, BTreeMap<u32, PcmTileRef>);

pub struct Files {
    directory: PathBuf,
    references: BTreeMap<u32, PcmTileRef>,
    pub reads: usize,
}
impl PcmTileStore for Files {
    fn lookup(
        &mut self,
        source: &Digest,
        index: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> mo_video_runtime::pcm::Result<Option<PcmTileRef>> {
        if cancelled() {
            return Err(PcmError::Cancelled);
        }
        Ok(self
            .references
            .get(&index)
            .filter(|r| &r.source == source)
            .cloned())
    }
    fn read(
        &mut self,
        reference: &PcmTileRef,
        max_bytes: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> mo_video_runtime::pcm::Result<Vec<u8>> {
        if cancelled() {
            return Err(PcmError::Cancelled);
        }
        if self.references.get(&reference.index) != Some(reference) {
            return Err(PcmError::Invalid("test tile identity"));
        }
        let file = File::open(self.directory.join(format!("{}.f32le", reference.index)))
            .map_err(|e| PcmError::Store(e.to_string()))?;
        let mut out = vec![];
        file.take(max_bytes as u64 + 1)
            .read_to_end(&mut out)
            .map_err(|e| PcmError::Store(e.to_string()))?;
        if out.len() > max_bytes {
            return Err(PcmError::Budget);
        }
        self.reads += 1;
        Ok(out)
    }
}
fn store(directory: &Path, tile: UnsealedPcmTile) -> UnsealedPcmTileRef {
    let mut file = File::options()
        .create_new(true)
        .write(true)
        .open(directory.join(format!("{}.f32le", tile.reference.index)))
        .unwrap();
    file.write_all(&tile.bytes).unwrap();
    file.sync_all().unwrap();
    tile.reference
}
pub fn decode(
    bundle: &WorkerBundle,
    path: &Path,
    directory: &Path,
) -> (InspectedPcm, CanonicalAudioResource<Files>, u32) {
    fs::create_dir(directory).unwrap();
    let container = NativeContainer::open_with_timeline(
        bundle,
        reader(path),
        fs::metadata(path).unwrap().len(),
        options(),
        DecodeTimeline::RawIsoBmff,
        &|| false,
    )
    .unwrap();
    let stream = container
        .streams()
        .iter()
        .find(|s| s.kind == MediaKind::Audio)
        .unwrap()
        .clone();
    let track = stream.container_track_id.unwrap();
    let mut decoder = container.select(stream.index, &|| false).unwrap();
    let origin = PcmStreamOrigin {
        resource: ResourceId::new("audio").unwrap(),
        content: Digest::from_sha256(Sha256::digest(fs::read(path).unwrap()).into()),
        stream: StreamId::new(format!("track-{track}")).unwrap(),
        decoder_profile: digest(
            "musteroffice.native-pcm-fixture/1",
            &(
                bundle.manifest_sha256(),
                "raw-isobmff",
                mo_video_runtime::pcm::PROFILE,
            ),
        )
        .unwrap(),
    };
    let limits = PcmStreamLimits {
        tile_frames: 4096,
        ..Default::default()
    };
    let mut builder = PcmStreamBuilder::new(origin.clone(), 1, limits).unwrap();
    let mut refs = vec![];
    while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
        for tile in builder.push(&frame, ByteOrder::Little, &|| false).unwrap() {
            refs.push(store(directory, tile));
        }
        assert!(
            builder.retained_bytes() <= 4096 * 4,
            "mono carry remains bounded"
        );
    }
    decoder.close(&|| false).unwrap();
    let mut completed = builder.finish_at_eof(&|| false).unwrap();
    if let Some(tail) = completed.tail.take() {
        refs.push(store(directory, tail));
    }
    assert_eq!(refs.len(), completed.source().tile_count() as usize);
    let references = refs
        .iter()
        .map(|r| {
            let r = completed.seal_ref(r).unwrap();
            (r.index, r)
        })
        .collect();
    let manifest: Manifest = (completed.record().unwrap(), references);
    let mut file = File::options()
        .create_new(true)
        .write(true)
        .open(directory.join("completion.json"))
        .unwrap();
    file.write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    drop(file);
    drop(manifest);
    drop(completed);
    // Reconstruct only from the saved completion and inventory, with no live
    // decoder, inspector or PcmSource retained. This fixture is not a crash or
    // authenticated product-store test; those are separate host requirements.
    let (record, references): Manifest =
        serde_json::from_slice(&fs::read(directory.join("completion.json")).unwrap()).unwrap();
    let (decoded, source) = record.restore(&origin, limits).unwrap();
    assert_eq!(references.len(), source.tile_count() as usize);
    for (index, reference) in &references {
        assert_eq!(*index, reference.index);
        source.validate_ref(reference).unwrap();
    }
    let files = Files {
        directory: directory.into(),
        references,
        reads: 0,
    };
    let resource = CanonicalAudioResource::new(
        Arc::clone(&source),
        files,
        PcmReadLimits {
            max_cached_tiles: 2,
            max_cached_bytes: 32768,
            ..Default::default()
        },
    )
    .unwrap();
    (decoded, resource, track)
}
