//! Isolated native shaping host. No input filenames; transport owns explicit bytes.
use mo_harfbuzz_sys::NativeShaper;
use std::io::{self, Read, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let execute = match args.as_slice() {
        [] => mo_kernel_api::shape_text_json,
        [mode] if mode == "--cascade" => mo_kernel_api::shape_cascade_json,
        [mode] if mode == "--paragraph" => mo_kernel_api::shape_paragraph_json,
        [mode] if mode == "--paths" => mo_kernel_api::paragraph_paths_json,
        [mode] if mode == "--layout" => mo_kernel_api::layout_paragraph_json,
        [mode] if mode == "--geometry" => mo_kernel_api::layout_lines_json,
        [mode] if mode == "--lines" => mo_kernel_api::shape_lines_json,
        [mode] if mode == "--outlines" => mo_kernel_api::outline_font_json,
        [mode] if mode == "--metrics" => mo_kernel_api::measure_font_json,
        _ => {
            return Err(
                "usage: mo-text-worker [--cascade|--paragraph|--lines|--metrics|--geometry|--layout|--outlines|--paths]".into(),
            );
        }
    };
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut backend = NativeShaper::default();
    loop {
        let mut header = [0u8; 8];
        match input.read(&mut header[..1])? {
            0 => break,
            1 => {}
            _ => unreachable!(),
        }
        input.read_exact(&mut header[1..])?;
        let request_length = u32::from_le_bytes(header[..4].try_into()?) as usize;
        let font_length = u32::from_le_bytes(header[4..].try_into()?) as usize;
        if request_length > mo_kernel_api::MAX_REQUEST_BYTES
            || font_length > mo_kernel_api::MAX_INLINE_FONT_BYTES
        {
            return Err("worker frame budget".into());
        }
        let mut request = vec![0; request_length];
        let mut font = vec![0; font_length];
        input.read_exact(&mut request)?;
        input.read_exact(&mut font)?;
        let response = execute(std::str::from_utf8(&request)?, &font, &mut backend, &|| {
            false
        });
        output.write_all(&(u32::try_from(response.len())?).to_le_bytes())?;
        output.write_all(response.as_bytes())?;
        output.flush()?;
        if backend.is_invalid() {
            break;
        }
    }
    Ok(())
}
