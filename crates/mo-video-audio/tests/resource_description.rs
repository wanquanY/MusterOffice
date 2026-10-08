mod support;
use mo_video_audio::*;
use support::*;

#[test]
fn wire_description_preserves_exact_lattice_and_rejects_stale_identity() {
    let (_, host) = document(vec![0.; 1024], 44100);
    let description =
        AudioResourceDescription::new(host.binding.clone(), &host.asset, 256).unwrap();
    let json = serde_json::to_string(&description).unwrap();
    let copy: AudioResourceDescription = mo_common::from_json_str_with_path(&json).unwrap();
    assert_eq!(copy.asset(&host.binding, 256).unwrap(), host.asset);
    let mut wrong = host.binding.clone();
    wrong.digest = mo_common::Digest::from_sha256([7; 32]);
    assert!(copy.asset(&wrong, 256).is_err());
    let mut wrong = copy.clone();
    wrong.end_sample = mo_common::Ticks::new(1023);
    assert!(wrong.asset(&host.binding, 256).is_err());
    wrong = copy.clone();
    wrong.sample_zero = mo_time::ExactRatio::integer(1);
    assert!(wrong.asset(&host.binding, 256).is_err());
    wrong = copy.clone();
    wrong.layout = vec![Channel::FrontLeft, Channel::FrontLeft];
    assert!(wrong.asset(&host.binding, 256).is_err());
    wrong = copy;
    wrong.sample_rate = 48000;
    assert!(wrong.asset(&host.binding, 256).is_err());
}
