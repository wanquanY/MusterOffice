use mo_opc::ResultSink;
use std::{
    error::Error,
    ffi::OsStr,
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub fn read_limited(path: &OsStr, max: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err("input must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err("input byte budget exceeded".into());
    }
    Ok(bytes)
}

pub(super) fn output_directory(output: &Path) -> &Path {
    output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

/// Stage/sync/read back and publish an immutable new artifact without overwrite.
pub(super) fn publish(
    bytes: &[u8],
    output: &OsStr,
    verify: impl FnOnce(File) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let output = Path::new(output);
    let mut sink = mo_native_io::FileSpool::create(output_directory(output), bytes.len() as u64)?;
    sink.write_all(bytes)?;
    let sealed = sink.seal()?;
    verify(sealed.reader.open_file()?)?;
    sealed.reader.link_new(output)?;
    Ok(())
}
