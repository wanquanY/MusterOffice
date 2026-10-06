use std::io::{Read, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024 + 1)
        .read_to_string(&mut input)?;
    let output = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => mo_video_conformance::video_conformance(&input),
        [mode] if mode == "--projection" => {
            mo_video_conformance::video_projection_conformance(&input)
        }
        [mode] if mode == "--operations" => {
            mo_video_conformance::video_operations_conformance(&input)
        }
        [mode] if mode == "--export" => mo_video_conformance::video_export_conformance(&input),
        [mode] if mode == "--render" => mo_video_conformance::video_render_conformance(&input),
        [mode] if mode == "--audio" => mo_video_conformance::video_audio_conformance(&input),
        [mode] if mode == "--pcm" => mo_video_conformance::video_pcm_conformance(&input),
        [mode] if mode == "--stretch" => mo_video_conformance::video_stretch_conformance(&input),
        [mode] if mode == "--retime" => mo_video_conformance::video_retime_conformance(&input),
        _ => {
            return Err(
                "usage: mo-video-conformance [--projection|--render|--audio|--pcm|--stretch|--retime|--export|--operations]"
                    .into(),
            );
        }
    };
    std::io::stdout().write_all(output.as_bytes())?;
    Ok(())
}
