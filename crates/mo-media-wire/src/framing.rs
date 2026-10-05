use crate::{MAX_HEADER_BYTES, PlaneLayout};
use mo_media::FramePlane;
use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, Read, Write};
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
/// EOF is valid only before the first byte of a new frame, never in its body.
pub fn read_header<T: DeserializeOwned>(input: &mut impl Read) -> io::Result<Option<T>> {
    let mut length = [0; 4];
    loop {
        match input.read(&mut length[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    input.read_exact(&mut length[1..])?;
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > MAX_HEADER_BYTES {
        return Err(invalid("media header byte budget"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| invalid("header allocation"))?;
    bytes.resize(length, 0);
    input.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}
struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_HEADER_BYTES - self.0.len() {
            return Err(invalid("media header byte budget"));
        }
        self.0
            .try_reserve(bytes.len())
            .map_err(|_| invalid("header allocation"))?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub fn header_bytes(header: &impl Serialize) -> io::Result<Vec<u8>> {
    let mut bytes = Bounded(Vec::new());
    serde_json::to_writer(&mut bytes, header).map_err(io::Error::other)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(bytes.0.len() + 4)
        .map_err(|_| invalid("header allocation"))?;
    result.extend_from_slice(&(bytes.0.len() as u32).to_le_bytes());
    result.extend(bytes.0);
    Ok(result)
}
pub fn write_header(output: &mut impl Write, header: &impl Serialize) -> io::Result<()> {
    output.write_all(&header_bytes(header)?)
}
/// Validate the entire layout before any plane allocation/read. No partial
/// frame escapes on budget, malformed shape, allocation or truncated payload.
pub fn read_planes(
    input: &mut impl Read,
    planes: &[PlaneLayout],
    max_bytes: u64,
) -> io::Result<Vec<FramePlane>> {
    let lengths = crate::plane_lengths(planes, max_bytes)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(planes.len())
        .map_err(|_| invalid("plane list allocation"))?;
    for (layout, length) in planes.iter().zip(lengths) {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| invalid("plane allocation"))?;
        bytes.resize(length, 0);
        input.read_exact(&mut bytes)?;
        result.push(FramePlane {
            row_bytes: layout.row_bytes as usize,
            rows: layout.rows as usize,
            bytes,
        });
    }
    Ok(result)
}
