mod support;
use mo_common::RationalTime;
use mo_time::ExactRatio as R;
use mo_video_compile::*;
use mo_video_model::*;
use support::*;
fn compile(d: VideoDocument) -> CompiledDocument {
    CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap()
}
fn at(n: i64) -> RationalTime {
    RationalTime::new(n, 16000).unwrap()
}
#[test]
fn span_projection_matches_point_oracle_at_every_boundary_and_sample() {
    let mut d = doc();
    d.audio_sample_rate = 8000;
    track(&mut d, "main", "audio", TrackKind::Audio);
    let mapping = TimeMap {
        anchors: vec![
            TimeAnchor {
                local: at(0),
                source: at(0),
            },
            TimeAnchor {
                local: at(13),
                source: at(40),
            },
            TimeAnchor {
                local: at(23),
                source: at(40),
            },
            TimeAnchor {
                local: at(43),
                source: at(0),
            },
        ],
    };
    let source = media(&mut d, true, mapping);
    clip(
        &mut d,
        "audio",
        "voice",
        TimeRange {
            start: at(3),
            end: at(46),
        },
        source,
    );
    let voice = d.clips.get_mut(&cid("voice")).unwrap();
    voice.audio.reverse = ReverseAudioPolicy::Reverse;
    voice.effects = vec![
        Effect::Gain {
            milli_decibels: -6000,
        },
        Effect::Fade {
            fade_in: at(10),
            fade_out: at(5),
            envelope: TimeRange {
                start: at(-3),
                end: at(43),
            },
        },
    ];
    let c = compile(d);
    for first in -5..25 {
        for frames in [1, 2, 7, 23] {
            let block = c
                .audio_sequence_block(&sid("main"), first, frames, QueryLimits::default(), &|| {
                    false
                })
                .unwrap()
                .value;
            for n in first..first + i64::from(frames) {
                let time = R::new(n.into(), 8000.into(), 256).unwrap();
                let point = c
                    .audio_at(
                        &TimeSample {
                            time,
                            side: SampleSide::After,
                        },
                        QueryLimits::default(),
                        &|| false,
                    )
                    .unwrap()
                    .value;
                let spans: Vec<_> = block
                    .voices
                    .iter()
                    .filter(|v| {
                        v.first_sample.get() <= n && n < v.first_sample.get() + i64::from(v.frames)
                    })
                    .collect();
                assert_eq!(spans.len(), point.len(), "sample {n}");
                for (span, voice) in spans.iter().zip(point) {
                    let elapsed =
                        R::new((n - span.first_sample.get()).into(), 8000.into(), 256).unwrap();
                    assert_eq!(span.clip, voice.clip);
                    assert_eq!(span.policy, voice.policy);
                    assert_eq!(span.mapping.rate, voice.mapping.rate);
                    assert_eq!(span.mapping.sample.side, voice.mapping.sample.side);
                    let mapped = span
                        .mapping
                        .sample
                        .time
                        .add(&elapsed.mul(&span.mapping.rate, 256).unwrap(), 256)
                        .unwrap();
                    assert_eq!(
                        mapped.compare(&voice.mapping.sample.time),
                        std::cmp::Ordering::Equal
                    );
                    assert_eq!(
                        span.local_start
                            .add(&elapsed, 256)
                            .unwrap()
                            .compare(&voice.local.time),
                        std::cmp::Ordering::Equal
                    );
                    assert!(
                        matches!(&span.effects[1],AudioSpanEffect::Fade{envelope,..} if envelope.start==at(-3))
                    );
                }
            }
        }
    }
}
#[test]
fn nested_pitch_stages_stay_lazy_and_filter_padding_is_silent() {
    let mut d = doc();
    sequence(&mut d, "child");
    track(&mut d, "main", "outer", TrackKind::Audio);
    track(&mut d, "child", "inner", TrackKind::Audio);
    let source = media(&mut d, true, map(0, 4, 2));
    clip(&mut d, "inner", "voice", range(0, 2), source);
    d.clips.get_mut(&cid("voice")).unwrap().audio.rate = AudioRatePolicy::Resample;
    clip(
        &mut d,
        "outer",
        "group",
        range(0, 4),
        nested("child", map(0, 2, 4)),
    );
    let c = compile(d);
    let outer = c
        .audio_block(0, 10, QueryLimits::default(), &|| false)
        .unwrap()
        .value;
    assert_eq!(
        outer.voices[0].mapping.rate,
        R::new(1.into(), 2.into(), 256).unwrap()
    );
    assert_eq!(outer.voices[0].policy.rate, AudioRatePolicy::PreservePitch);
    assert!(
        matches!(&outer.voices[0].source,AudioSpanSource::Sequence{sequence} if sequence==&sid("child"))
    );
    let child = c
        .audio_sequence_block(&sid("child"), -10, 20, QueryLimits::default(), &|| false)
        .unwrap()
        .value;
    assert_eq!(child.voices[0].first_sample.get(), 0);
    assert_eq!(child.voices[0].frames, 10);
    assert_eq!(child.voices[0].policy.rate, AudioRatePolicy::Resample);
    assert!(
        c.audio_sequence_block(&sid("child"), 96000, 100, QueryLimits::default(), &|| false)
            .unwrap()
            .value
            .voices
            .is_empty()
    );
    assert!(
        c.audio_block(-1, 1, QueryLimits::default(), &|| false)
            .is_err()
    );
    assert!(
        c.audio_block(0, 65537, QueryLimits::default(), &|| false)
            .is_err()
    );
}
