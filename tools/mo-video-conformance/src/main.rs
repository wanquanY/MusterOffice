use std::io::{Read, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024 + 1)
        .read_to_string(&mut input)?;
    let output = mo_video_conformance::video_conformance(&input);
    std::io::stdout().write_all(output.as_bytes())?;
    Ok(())
}
