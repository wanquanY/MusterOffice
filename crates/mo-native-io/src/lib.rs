//! Native host output staging shared by CLI and standard/embedded hosts. This is
//! not an Artifact store or a commit owner. The pure kernel never depends on it.
mod directory;
pub use directory::SpoolDirectory;
use mo_opc::{ReaderAt, ResultSink, SealedOutput};
use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const BUFFER_BYTES: usize = 65_536;
static NEXT_SPOOL: AtomicU64 = AtomicU64::new(0);

/// The operator supplies a private, protected directory. No client path is
/// accepted by this capability. Each sink creates an exclusive child directory
/// and file; failures and drops remove only that owned file/empty directory.
///
/// Bounds logical output bytes and buffers 64 KiB. The job scheduler must still
/// reserve aggregate disk usage, track crash orphans and enforce process limits.
pub struct FileSpool {
    writer: Option<BufWriter<File>>,
    location: Option<Location>,
    length: u64,
    limit: u64,
    failed: bool,
}

/// Read-only owned capability over sealed bytes. It exposes no path, writer,
/// mutable file handle or clone. It remains private until a host commit succeeds.
pub struct SealedFile {
    reader: Option<rawzip::FileReader>,
    location: Option<Location>,
    length: u64,
}

struct Location {
    directory: PathBuf,
    file: PathBuf,
}

impl Location {
    fn remove(&self) -> io::Result<()> {
        match fs::remove_file(&self.file) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
        match fs::remove_dir(&self.directory) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl FileSpool {
    pub fn create(operator_root: &Path, max_bytes: u64) -> io::Result<Self> {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        for _ in 0..128 {
            let nonce = NEXT_SPOOL.fetch_add(1, Ordering::Relaxed);
            let directory =
                operator_root.join(format!("mo-spool-{}-{epoch}-{nonce}", std::process::id()));
            let mut builder = DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&directory) {
                Ok(()) => (),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
            let location = Location {
                file: directory.join("output"),
                directory,
            };
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let file = match options.open(&location.file) {
                Ok(file) => file,
                Err(e) => {
                    // Do not remove a file that this call did not create.
                    let _ = fs::remove_dir(&location.directory);
                    return Err(e);
                }
            };
            return Ok(Self {
                writer: Some(BufWriter::with_capacity(BUFFER_BYTES, file)),
                location: Some(location),
                length: 0,
                limit: max_bytes,
                failed: false,
            });
        }
        Err(io::Error::other(
            "cannot allocate an exclusive output spool",
        ))
    }

    fn writable(&self) -> io::Result<()> {
        if self.failed {
            Err(io::Error::other(
                "output spool is invalid after an earlier failure",
            ))
        } else {
            Ok(())
        }
    }

    fn close(&mut self) {
        if let Some(writer) = self.writer.take() {
            // Abandon pending buffer bytes without a surprise Drop flush.
            drop(writer.into_parts());
        }
    }

    /// Explicit cleanup reports errors. Drop cleanup is best effort, so the
    /// durable owner must also account for process-crash and cleanup orphans.
    pub fn discard(mut self) -> io::Result<()> {
        self.close();
        self.location.as_ref().expect("owned location").remove()?;
        self.location.take();
        Ok(())
    }
}

impl Write for FileSpool {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writable()?;
        if bytes.len() as u64 > self.limit - self.length {
            self.failed = true;
            return Err(io::Error::other("output spool byte budget exceeded"));
        }
        match self.writer.as_mut().expect("writable spool").write(bytes) {
            Ok(n) => {
                self.length += n as u64;
                Ok(n)
            }
            Err(e) => {
                self.failed = true;
                Err(e)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writable()?;
        if let Err(e) = self.writer.as_mut().expect("writable spool").flush() {
            self.failed = true;
            return Err(e);
        }
        Ok(())
    }
}

impl ResultSink for FileSpool {
    type Reader = SealedFile;

    fn seal(mut self) -> io::Result<SealedOutput<Self::Reader>> {
        self.flush()?;
        let file = self.writer.as_ref().expect("writable spool").get_ref();
        file.sync_all()?;
        if file.metadata()?.len() != self.length {
            return Err(io::Error::other("stored spool length differs from writes"));
        }
        self.close();
        let file = File::open(&self.location.as_ref().expect("owned location").file)?;
        let actual = file.metadata()?.len();
        if actual != self.length {
            return Err(io::Error::other("sealed spool length changed"));
        }
        Ok(SealedOutput {
            reader: SealedFile {
                reader: Some(file.into()),
                location: self.location.take(),
                length: actual,
            },
            byte_length: actual,
        })
    }
}

impl Drop for FileSpool {
    fn drop(&mut self) {
        self.close();
        if let Some(location) = &self.location {
            let _ = location.remove();
        }
    }
}

impl SealedFile {
    pub fn byte_length(&self) -> u64 {
        self.length
    }

    /// A fresh read-only handle for host validators needing std::fs::File.
    pub fn open_file(&self) -> io::Result<File> {
        File::open(&self.location.as_ref().expect("owned location").file)
    }

    /// Authorized local host publication: link the same sealed inode at a new
    /// destination, atomically refusing overwrite. Never falls back to copying
    /// unchecked bytes or crossing filesystems. This is not a product job commit
    /// or a guarantee that directory entries survive sudden power loss.
    pub fn link_new(&self, operator_destination: &Path) -> io::Result<()> {
        fs::hard_link(
            &self.location.as_ref().expect("owned location").file,
            operator_destination,
        )
    }

    pub fn discard(mut self) -> io::Result<()> {
        self.reader.take();
        self.location.as_ref().expect("owned location").remove()?;
        self.location.take();
        Ok(())
    }
}

impl ReaderAt for SealedFile {
    fn read_at(&self, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
        if offset >= self.length || buffer.is_empty() {
            return Ok(0);
        }
        let n = (self.length - offset).min(buffer.len() as u64) as usize;
        self.reader
            .as_ref()
            .expect("sealed reader")
            .read_at(&mut buffer[..n], offset)
    }
}

impl Drop for SealedFile {
    fn drop(&mut self) {
        // Close before unlink for Windows as well as Unix.
        self.reader.take();
        if let Some(location) = &self.location {
            let _ = location.remove();
        }
    }
}
