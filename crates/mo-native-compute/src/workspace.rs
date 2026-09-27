//! A deliberately narrow local file bridge. Only portable leaf names are
//! accepted; roots/ancestors remain protected and stable by the calling host.
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Directory {
    path: PathBuf,
}

pub fn leaf(name: &str) -> io::Result<()> {
    let bytes = name.as_bytes();
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if bytes.is_empty()
        || bytes.len() > 128
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(c))
        || name.ends_with('.')
        || ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "portable leaf name required",
        ));
    }
    Ok(())
}

fn redirected(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

impl Directory {
    /// Host configuration only, never a tool argument. The host must prevent
    /// concurrent root/ancestor renames and external mutation of active output
    /// directories. This adapter does not build a filesystem permission system.
    pub fn open(path: &Path) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_dir() || redirected(&metadata) {
            return Err(io::Error::other("file bridge requires a real directory"));
        }
        Ok(Self {
            path: fs::canonicalize(path)?,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn child(&self, name: &str) -> io::Result<PathBuf> {
        leaf(name)?;
        Ok(self.path.join(name))
    }
    pub fn directory(&self, name: &str) -> io::Result<Self> {
        Self::open(&self.child(name)?)
    }
    pub fn read(&self, name: &str) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            );
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
        }
        let file = options.open(self.child(name)?)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || redirected(&metadata) {
            return Err(io::Error::other("regular input file required"));
        }
        Ok(file)
    }
}
