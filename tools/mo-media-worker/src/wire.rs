use mo_media_wire::{
    Command, Event, MAX_READ_BYTES, MAX_WRITE_BYTES, Request, Response, read_header, write_header,
};
use std::io::{self, BufReader, BufWriter, Read, Write};
pub struct Wire {
    pub input: BufReader<io::Stdin>,
    output: BufWriter<io::Stdout>,
    pub call_id: u64,
    read_id: u64,
    write_id: u64,
}
impl Wire {
    pub fn new() -> Self {
        Self {
            input: BufReader::new(io::stdin()),
            output: BufWriter::new(io::stdout()),
            call_id: 0,
            read_id: 0,
            write_id: 0,
        }
    }
    pub fn respond<'a>(
        &mut self,
        event: &Event,
        planes: impl Iterator<Item = &'a [u8]>,
    ) -> io::Result<()> {
        // Serialize a borrowed envelope without duplicating metadata/payloads.
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Borrowed<'a> {
            call_id: u64,
            event: &'a Event,
        }
        write_header(
            &mut self.output,
            &Borrowed {
                call_id: self.call_id,
                event,
            },
        )?;
        for bytes in planes {
            self.output.write_all(bytes)?;
        }
        self.output.flush()
    }
    pub fn read_source(&mut self, offset: u64, destination: &mut [u8]) -> io::Result<usize> {
        // AVIO can bypass its internal buffer for a large packet. Return a
        // bounded short read, rather than rejecting valid large media packets.
        let length = u32::try_from(destination.len().min(MAX_READ_BYTES as usize))
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| io::Error::other("resource read capacity"))?;
        self.read_id = self
            .read_id
            .checked_add(1)
            .ok_or_else(|| io::Error::other("resource read id exhausted"))?;
        write_header(
            &mut self.output,
            &Response {
                call_id: self.call_id,
                event: Event::Read {
                    read_id: self.read_id,
                    offset,
                    byte_length: length,
                },
            },
        )?;
        self.output.flush()?;
        let response: Request = read_header(&mut self.input)?
            .ok_or_else(|| io::Error::other("resource reply missing"))?;
        match response.command {
            Command::ReadReply {
                read_id,
                byte_length,
            } if response.call_id == self.call_id
                && read_id == self.read_id
                && byte_length > 0
                && byte_length <= length =>
            {
                self.input
                    .read_exact(&mut destination[..byte_length as usize])?;
                Ok(byte_length as usize)
            }
            _ => Err(io::Error::other("invalid resource reply")),
        }
    }
    pub fn write_candidate(&mut self, offset: u64, bytes: &[u8]) -> io::Result<usize> {
        let n = bytes.len().min(MAX_WRITE_BYTES as usize);
        if n == 0 {
            return Err(io::Error::other("empty candidate write"));
        }
        self.write_id = self
            .write_id
            .checked_add(1)
            .ok_or_else(|| io::Error::other("write id exhausted"))?;
        self.respond(
            &Event::Write {
                write_id: self.write_id,
                offset,
                byte_length: n as u32,
            },
            std::iter::once(&bytes[..n]),
        )?;
        let reply: Request =
            read_header(&mut self.input)?.ok_or_else(|| io::Error::other("write reply missing"))?;
        match reply.command {
            Command::WriteReply {
                write_id,
                byte_length,
            } if reply.call_id == self.call_id
                && write_id == self.write_id
                && byte_length > 0
                && byte_length <= n as u32 =>
            {
                Ok(byte_length as usize)
            }
            _ => Err(io::Error::other("invalid candidate write reply")),
        }
    }
}
