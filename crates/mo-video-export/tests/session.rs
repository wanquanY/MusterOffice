mod support;
use mo_media::*;
use mo_video_edit::{
    EditContract, EditLimits, Operation, Snapshot, Transaction, prepare, prepare_inverse,
};
use mo_video_export::*;
use mo_video_model::*;
use std::{cell::RefCell, rc::Rc};
use support::*;
#[derive(Default)]
struct Captured {
    frames: Vec<(i64, i64, Vec<u8>)>,
    samples: Vec<f32>,
    finished: usize,
    drops: usize,
    fail: bool,
}
struct Recorder {
    config: EncodeConfig,
    out: Rc<RefCell<Captured>>,
}
impl Drop for Recorder {
    fn drop(&mut self) {
        self.out.borrow_mut().drops += 1;
    }
}
impl VideoEncoder for Recorder {
    type Error = std::io::Error;
    fn configuration(&self) -> EncodeConfig {
        self.config
    }
    fn video(&mut self, f: EncodeVideoFrame<'_>) -> Result<(), Self::Error> {
        let mut out = self.out.borrow_mut();
        if out.fail {
            return Err(std::io::Error::other("codec failure"));
        }
        out.frames.push((
            f.pts,
            f.duration,
            f.planes.iter().flat_map(|p| p.iter().copied()).collect(),
        ));
        Ok(())
    }
    fn audio(&mut self, first: i64, samples: &[f32]) -> Result<(), Self::Error> {
        let mut out = self.out.borrow_mut();
        assert_eq!(first, out.samples.len() as i64 / 2);
        out.samples.extend(samples);
        Ok(())
    }
    fn finish(&mut self) -> Result<EncodeStats, Self::Error> {
        let mut out = self.out.borrow_mut();
        out.finished += 1;
        Ok(EncodeStats {
            byte_length: 1,
            write_bytes: 1,
            video_frames: out.frames.len() as u64,
            video_packets: out.frames.len() as u64,
            audio_samples: out.samples.len() as u64 / 2,
            audio_packets: 1,
            peak_pending_bytes: 0,
        })
    }
}
fn run(
    document: &mo_video_compile::CompiledDocument,
    chunk: u32,
) -> (ExportCandidate, Rc<RefCell<Captured>>) {
    let plan = ExportPlan::new(
        document,
        options(),
        ExportLimits {
            audio_block_frames: chunk,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    let out = Rc::new(RefCell::new(Captured::default()));
    let mut session = ExportSession::new(
        plan,
        Recorder {
            config: ExportPlan::new(document, options(), ExportLimits::default(), &|| false)
                .unwrap()
                .configuration(),
            out: out.clone(),
        },
    )
    .unwrap();
    let mut audio = Audio::default();
    loop {
        match session.step(&mut Visual, &mut audio, &|| false).unwrap() {
            ExportStep::Advanced(_) => (),
            ExportStep::WaitingForAudio(_) => panic!("unexpected preparation"),
            ExportStep::ReadyToVerify(candidate) => {
                assert!(session.is_closed());
                assert!(matches!(
                    session.step(&mut Visual, &mut audio, &|| false),
                    Err(ExportError::State)
                ));
                return (*candidate, out);
            }
        }
    }
}
#[test]
fn absolute_clocks_gap_matte_and_arbitrary_audio_blocks_match() {
    let d = compile(document());
    let (receipt, a) = run(&d, 997);
    let (_, b) = run(&d, 4093);
    let a = a.borrow();
    let b = b.borrow();
    assert_eq!(a.frames, b.frames);
    assert_eq!(a.samples, b.samples);
    assert_eq!(a.frames.len(), 16);
    assert_eq!(a.samples.len(), 24157 * 2);
    assert_eq!(receipt.configuration.video_time_scale, 240000);
    assert_eq!(a.frames[15].0, 120120);
    assert_eq!(a.frames[15].1, 665);
    assert_eq!(a.frames[0].2[0], 63); // opaque red
    assert_eq!(a.frames[12].2[0], 47); // explicit dark matte in authored gap
    assert_eq!(a.frames[14].2[0], 173); // green after the gap
    assert_eq!(a.finished, 1);
    assert_eq!(a.drops, 1);
    assert_eq!(receipt.document_digest, *d.digest());
}
#[test]
fn pending_audio_retains_progress_but_failure_cancel_and_binding_abort() {
    let d = compile(document());
    let plan = ExportPlan::new(&d, options(), ExportLimits::default(), &|| false).unwrap();
    let out = Rc::new(RefCell::new(Captured::default()));
    let config = plan.configuration();
    let mut session = ExportSession::new(
        plan,
        Recorder {
            config,
            out: out.clone(),
        },
    )
    .unwrap();
    let mut audio = Audio::default();
    audio.pending = true;
    assert!(matches!(
        session.step(&mut Visual, &mut audio, &|| false).unwrap(),
        ExportStep::Advanced(_)
    ));
    let progress = session.progress();
    for _ in 0..2 {
        assert!(matches!(
            session.step(&mut Visual, &mut audio, &|| false).unwrap(),
            ExportStep::WaitingForAudio(_)
        ));
        assert_eq!(session.progress(), progress);
    }
    audio.pending = false;
    assert!(matches!(
        session.step(&mut Visual, &mut audio, &|| false).unwrap(),
        ExportStep::Advanced(_)
    ));
    assert!(session.progress().audio_samples > 0);
    assert!(matches!(
        session.step(&mut Visual, &mut audio, &|| true),
        Err(ExportError::Cancelled)
    ));
    assert!(session.is_closed());
    assert_eq!(out.borrow().drops, 1);
    assert_eq!(out.borrow().finished, 0);
    let mut wrong = config;
    wrong.video_bit_rate += 1;
    assert!(matches!(
        ExportSession::new(
            ExportPlan::new(&d, options(), ExportLimits::default(), &|| false).unwrap(),
            Recorder {
                config: wrong,
                out: out.clone()
            }
        ),
        Err(ExportError::Binding)
    ));
    let out = Rc::new(RefCell::new(Captured {
        fail: true,
        ..Default::default()
    }));
    let mut session = ExportSession::new(
        ExportPlan::new(&d, options(), ExportLimits::default(), &|| false).unwrap(),
        Recorder {
            config,
            out: out.clone(),
        },
    )
    .unwrap();
    assert!(matches!(
        session.step(&mut Visual, &mut audio, &|| false),
        Err(ExportError::Encode(_))
    ));
    assert_eq!(out.borrow().drops, 1);
    assert_eq!(out.borrow().finished, 0);
}
#[test]
fn accepted_edit_and_inverse_change_export_while_previous_snapshot_remains_stable() {
    let snapshot = Snapshot::new(document(), ValidationLimits::default(), &|| false).unwrap();
    let compiled = compile(snapshot.document().clone());
    let edited = prepare(
        &snapshot,
        &Transaction {
            contract: EditContract::V1,
            expected_digest: snapshot.digest().clone(),
            operations: vec![
                Operation::ReplaceSource {
                    clip: ClipId::new("red").unwrap(),
                    source: ClipSource::Solid { rgba: [255; 4] },
                },
                Operation::SetEffects {
                    clip: ClipId::new("sound").unwrap(),
                    effects: vec![],
                },
            ],
        },
        EditLimits::default(),
        &|| false,
    )
    .unwrap();
    let changed = compile(edited.candidate.document().clone());
    let restored = prepare_inverse(
        &edited.candidate,
        &edited.inverse,
        EditLimits::default(),
        &|| false,
    )
    .unwrap();
    let original = run(&compiled, 997).1;
    let after = run(&changed, 997).1;
    let undo = run(&compile(restored.candidate.into_document()), 997).1;
    assert_eq!(original.borrow().frames, undo.borrow().frames);
    assert_eq!(original.borrow().samples, undo.borrow().samples);
    assert_eq!(after.borrow().frames[0].2[0], 235);
    assert_ne!(original.borrow().samples, after.borrow().samples);
    assert_eq!(compiled.digest(), snapshot.digest());
}
#[test]
fn inexpressible_clocks_odd_dimensions_and_output_budgets_are_explicit() {
    let d = compile(document());
    for limits in [
        ExportLimits {
            max_video_frames: 1,
            ..Default::default()
        },
        ExportLimits {
            max_audio_samples: 1,
            ..Default::default()
        },
        ExportLimits {
            max_video_output_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            ExportPlan::new(&d, options(), limits, &|| false),
            Err(PlanError::Budget)
        ));
    }
    let mut odd = document();
    odd.canvas.width = 127;
    assert!(matches!(
        ExportPlan::new(&compile(odd), options(), ExportLimits::default(), &|| false),
        Err(PlanError::Invalid(_))
    ));
    let mut huge = document();
    huge.frame_rate = FrameRate {
        numerator: 999999937,
        denominator: 33333331,
    };
    assert!(matches!(
        ExportPlan::new(&compile(huge), options(), ExportLimits::default(), &|| {
            false
        }),
        Err(PlanError::Invalid(_))
    ));
    let mut fractional = document();
    fractional.audio_sample_rate = 44100;
    let c = compile(fractional);
    let plan = ExportPlan::new(&c, options(), ExportLimits::default(), &|| false).unwrap();
    assert!(plan.audio_tail().positive());
    assert!(
        plan.audio_tail()
            .compare(&mo_time::ExactRatio::from_time(
                mo_common::RationalTime::new(1, 44100).unwrap()
            ))
            .is_lt()
    );
}
