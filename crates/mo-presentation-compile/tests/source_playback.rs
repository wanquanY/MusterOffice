#[allow(dead_code)]
#[path = "../../../tools/test-support/source_playback.rs"]
mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits};
use mo_pptx::source::SourceLimits;
use mo_presentation_compile::{source_page::*, source_playback::*, source_resource_page};
use mo_timeline::*;
use support::*;
fn binding(bytes: &[u8]) -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("source-test").unwrap(),
        generation: PlaybackGeneration::new(3),
        revision: read(bytes).source_sha256,
    }
}
fn time(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
struct Never;
impl mo_image::ImageDecoder for Never {
    fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        panic!("unexpected decoder")
    }
    fn invalidate(&mut self) {
        panic!("unexpected invalidation")
    }
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
#[test]
fn source_integer_animation_matches_declared_transform_and_fraction_stays_exact() {
    let base = image_fixture(&drawing_shape(42, &solid("2255EE")));
    let bytes = animated(&base, 42, 0, 21600000, "remove");
    let p = package(&bytes);
    let index = read(&bytes);
    let before = serde_json::to_value(&index).unwrap();
    let mut plan = SourcePlaybackPlan::new(
        &p,
        &index,
        request(&index),
        binding(&bytes),
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    for (n, d, angle) in [
        (0, 1, 0),
        (1, 4, 5400000),
        (1, 2, 10800000),
        (3, 4, 16200000),
        (1, 1, 0),
        (1, 4, 5400000),
    ] {
        let sample = plan.sample(time(n, d), None, &|| false).unwrap();
        let result = sample
            .prepare(&p, &index, &mut Never, None, options(), &|| false)
            .unwrap()
            .plan(&|| false)
            .unwrap();
        let control = rewrite(&base, SLIDE, |s| {
            s.replace("<a:xfrm >", &format!("<a:xfrm rot=\"{angle}\">"))
        });
        let ip = package(&control);
        let ii = read(&control);
        let expected = source_resource_page::prepare(
            &ip,
            &ii,
            &request(&ii),
            &mut Never,
            None,
            options(),
            &|| false,
        )
        .unwrap()
        .plan(&|| false)
        .unwrap();
        assert_eq!(
            serde_json::to_value(result.page.raster).unwrap(),
            serde_json::to_value(expected.page.raster).unwrap()
        );
    }
    let sample = plan.sample(time(1, 7), None, &|| false).unwrap();
    assert_eq!(
        sample.frame().evaluated.state.rotations[&ObjectId::new("sp.42").unwrap()].denominator,
        "7"
    );
    assert_eq!(
        sample.frame().object_bindings[&ObjectId::new("sp.42").unwrap()].part,
        SLIDE
    );
    assert_eq!(serde_json::to_value(index).unwrap(), before);
}
#[test]
fn sample_cannot_be_prepared_against_another_source_or_stale_binding() {
    let base = image_fixture(&drawing_shape(42, &solid("2255EE")));
    let bytes = animated(&base, 42, 0, 21600000, "freeze");
    let p = package(&bytes);
    let index = read(&bytes);
    let q = request(&index);
    let mut b = binding(&bytes);
    b.revision = Digest::from_sha256([0; 32]);
    assert!(matches!(
        SourcePlaybackPlan::new(
            &p,
            &index,
            q.clone(),
            b,
            SourceLimits::default(),
            TimelineLimits::default(),
            &|| false
        ),
        Err(SourcePlaybackError::RevisionConflict)
    ));
    let mut plan = SourcePlaybackPlan::new(
        &p,
        &index,
        q,
        binding(&bytes),
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let sample = plan.sample(time(1, 7), None, &|| false).unwrap();
    let other = package(&base);
    let oi = read(&base);
    assert!(matches!(
        sample.prepare(&other, &index, &mut Never, None, options(), &|| false),
        Err(SourcePageError::SourceConflict)
    ));
    assert!(matches!(
        sample.prepare(&other, &oi, &mut Never, None, options(), &|| false),
        Err(SourcePageError::SourceConflict)
    ));
    assert!(
        sample
            .prepare(&p, &index, &mut Never, None, options(), &|| true)
            .is_err()
    );
    assert!(matches!(
        plan.sample(time(1, 7), None, &|| true),
        Err(SourcePlaybackError::Timeline(TimelineError::Cancelled))
    ));
}
#[test]
fn missing_timing_is_static_and_unknown_native_timing_is_never_ignored() {
    let base = image_fixture(&drawing_shape(42, &solid("2255EE")));
    let p = package(&base);
    let index = read(&base);
    let mut plan = SourcePlaybackPlan::new(
        &p,
        &index,
        request(&index),
        binding(&base),
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(
        plan.sample(time(23, 7), None, &|| false)
            .unwrap()
            .frame()
            .evaluated
            .state
            .rotations
            .is_empty()
    );
    let bad = animated(&base, 42, 0, 21600000, "freeze");
    let bad = rewrite(&bad, SLIDE, |s| {
        s.replace(
            "restart=\"never\" fill=\"freeze\"",
            "restart=\"always\" fill=\"freeze\"",
        )
    });
    let p = package(&bad);
    let index = read(&bad);
    assert!(matches!(
        SourcePlaybackPlan::new(
            &p,
            &index,
            request(&index),
            binding(&bad),
            SourceLimits::default(),
            TimelineLimits::default(),
            &|| false
        ),
        Err(SourcePlaybackError::Page(SourcePageError::Source(
            mo_pptx::PptxError::Unsupported(_)
        )))
    ));
}
