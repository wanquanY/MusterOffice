mod support;
use mo_video_audio::{AudioError, AudioResources, PcmLimits, PcmRange};
use mo_video_pcm::{CanonicalAudioResource, PcmReadLimits};
use std::sync::Arc;
use support::*;
#[test]
fn pending_source_is_structured_and_becomes_readable_after_publication() {
    let source = source(0, 1024, 256);
    let mut resource = CanonicalAudioResource::new(
        Arc::clone(&source),
        Memory::default(),
        PcmReadLimits::default(),
    )
    .unwrap();
    let asset = resource.describe(source.binding()).unwrap();
    let range = PcmRange::new(500, 600).unwrap();
    assert!(
        matches!(resource.read(source.binding(),&asset,range,PcmLimits::default(),&||false),Err(AudioError::PendingSource{resource:id,first_sample:500,end_sample:600}) if id==source.binding().resource)
    );
    *resource.store_mut() = materialize(Arc::clone(&source), &[37, 997]);
    let pcm = resource
        .read(
            source.binding(),
            &asset,
            range,
            PcmLimits::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(pcm.samples(), block(500, 600).samples());
}
#[test]
fn mixer_request_budget_is_checked_before_storage_access() {
    let source = source(0, 1024, 256);
    let mut resource = CanonicalAudioResource::new(
        Arc::clone(&source),
        Memory::default(),
        PcmReadLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        resource.read(
            source.binding(),
            source.asset(),
            PcmRange::new(0, 100).unwrap(),
            PcmLimits {
                max_frames: 99,
                max_bytes: 400
            },
            &|| false
        ),
        Err(AudioError::Budget)
    ));
    assert_eq!(resource.store_mut().lookups, 0);
    assert_eq!(resource.store_mut().reads, 0);
}
