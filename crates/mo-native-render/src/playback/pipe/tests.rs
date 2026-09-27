use super::*;
use serde_json::{Value, json};
use std::process::Command;
fn pipe(bytes: &[u8]) -> Pipe<Value> {
    let encoded: String = bytes.iter().map(|b| format!("\\{b:03o}")).collect();
    let mut command = Command::new("/bin/sh");
    command.args(["-c", &format!("printf '{encoded}'; exec /bin/sleep 30")]);
    Pipe {
        session: Session::start(command, read_frame).unwrap(),
        timeout: Duration::from_secs(2),
        resources: false,
        first_call: None,
        deadline: Instant::now() + Duration::from_secs(2),
    }
}
fn packet(json: &[u8], pixels: &[u8]) -> Vec<u8> {
    [
        (json.len() as u32).to_le_bytes().as_slice(),
        (pixels.len() as u32).to_le_bytes().as_slice(),
        json,
        pixels,
    ]
    .concat()
}
fn info(pixels: &[u8]) -> RasterInfo {
    serde_json::from_value(json!({"profile":mo_raster::PROFILE,"width":1,"height":1,"byteLength":"4",
        "sha256":Digest::from_sha256(Sha256::digest(pixels).into()),"frameSha256":"0".repeat(64),
        "work":{"paths":0,"commands":0,"draws":0,"drawnCommands":0,"coordinateErrorBound":"0","strokeStyles":0,"strokeDraws":0,"strokeWidthErrorBound":"0","miterLimitErrorBound":"0","gradients":0,"gradientStops":0,"gradientDraws":0,"gradientCoordinateErrorBound":"0","gradientValueErrorBound":0.0}})).unwrap()
}
#[test]
fn malformed_wire_limits_and_control_pixels_stop_the_owner() {
    for bytes in [
        packet(br#"{"a":1,"a":2}"#, &[]),
        packet(b"!", &[]),
        [(64u32 * 1024 * 1024 + 1).to_le_bytes(), 0u32.to_le_bytes()].concat(),
        [
            0u32.to_le_bytes(),
            (mo_raster::MAX_PIXEL_BYTES as u32 + 1).to_le_bytes(),
        ]
        .concat(),
    ] {
        let mut p = pipe(&bytes);
        assert!(p.control(&json!({}), &|| false).is_err());
        assert!(p.is_stopped());
    }
    let mut p = pipe(&packet(b"{}", &[1]));
    assert!(matches!(
        p.control(&json!({}), &|| false),
        Err(Error::Response(_))
    ));
    assert!(p.is_stopped());
}
#[test]
fn validation_uses_the_shared_raster_profile_registry_and_resource_mode() {
    let pixels = [0, 0, 0, 255];
    for images in [false, true] {
        for version in 4..=12 {
            let profile = mo_raster::profile_for_frame(version).unwrap();
            let mut p = pipe(&[]);
            p.resources = images;
            let mut info = info(&pixels);
            info.profile = profile.into();
            assert_eq!(
                p.validate_pixels(&info, &pixels, (1, 1), &|| false).is_ok(),
                mo_raster::accepts_profile(profile, images)
            );
        }
    }
}
#[test]
fn wrong_pixel_bytes_dimensions_and_alpha_never_escape() {
    let pixels = [10, 20, 30, 255];
    let original = info(&pixels);
    for case in 0..5 {
        let mut p = pipe(&[]);
        let mut info = original.clone();
        let mut actual = pixels;
        match case {
            0 => actual[0] ^= 1,
            1 => info.width = 2,
            2 => info.byte_length = mo_common::ByteLength::new(3),
            3 => {
                actual[3] = 0;
                info.sha256 = Digest::from_sha256(Sha256::digest(actual).into());
            }
            _ => info.profile = "other".into(),
        }
        assert!(
            p.validate_pixels(&info, &actual, (1, 1), &|| false)
                .is_err()
        );
        assert!(p.is_stopped());
    }
    let mut p = pipe(&[]);
    p.validate_pixels(&original, &pixels, (1, 1), &|| false)
        .unwrap();
    assert!(!p.is_stopped());
    assert!(
        p.validate_pixels(&original, &pixels, (u32::MAX, u32::MAX), &|| false)
            .is_err()
    );
    assert!(p.is_stopped());
}
#[test]
fn cancellation_and_deadline_cover_final_pixel_validation() {
    let pixels = [0, 0, 0, 255];
    let info = info(&pixels);
    let mut p = pipe(&[]);
    assert!(matches!(
        p.validate_pixels(&info, &pixels, (1, 1), &|| true),
        Err(Error::Transport(SessionError::Cancelled))
    ));
    assert!(p.is_stopped());
    let mut p = pipe(&[]);
    p.deadline = Instant::now() - Duration::from_millis(1);
    assert!(matches!(
        p.validate_pixels(&info, &pixels, (1, 1), &|| false),
        Err(Error::Transport(SessionError::Deadline))
    ));
    assert!(p.is_stopped());
}
