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
        [mode] if mode == "--render" => mo_video_conformance::video_render_conformance(&input),
        _ => return Err("usage: mo-video-conformance [--projection|--render]".into()),
    };
    std::io::stdout().write_all(output.as_bytes())?;
    Ok(())
}
