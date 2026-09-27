use mo_common::{ByteLength, Digest};
use mo_opc::ReaderAt;
use mo_presentation_operations::{
    AssetDescriptor, AssetId, AssetInfo, AssetVerification, ExportAsset, ExportAssets, Failure,
    FailureCode,
};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, io};

/// Borrowed input bytes/readers and bounded metadata; this is not an asset store.
/// Inputs remain owned by the caller for the lifetime of the operation.
#[derive(Default)]
pub struct Inputs<'a> {
    entries: BTreeMap<AssetId, Entry<'a>>,
}
struct Entry<'a> {
    info: AssetInfo,
    reader: Input<'a>,
}
enum Input<'a> {
    Bytes(&'a [u8]),
    Reader(&'a dyn ReaderAt),
}
impl ReaderAt for Input<'_> {
    fn read_at(&self, output: &mut [u8], offset: u64) -> io::Result<usize> {
        match self {
            Self::Bytes(bytes) => bytes.read_at(output, offset),
            Self::Reader(reader) => reader.read_at(output, offset),
        }
    }
}
impl<'a> Inputs<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_bytes(
        &mut self,
        id: AssetId,
        media_type: impl Into<String>,
        bytes: &'a [u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), Failure> {
        let media_type = media_type.into();
        self.admit(&id, &media_type)?;
        let mut hash = Sha256::new();
        for chunk in bytes.chunks(65536) {
            if cancelled() {
                return Err(Failure::new(
                    FailureCode::Cancelled,
                    "input hashing cancelled",
                ));
            }
            hash.update(chunk);
        }
        if cancelled() {
            return Err(Failure::new(
                FailureCode::Cancelled,
                "input hashing cancelled",
            ));
        }
        let info = AssetInfo {
            id,
            descriptor: AssetDescriptor {
                sha256: Digest::from_sha256(hash.finalize().into()),
                byte_length: ByteLength::new(bytes.len() as u64),
                media_type,
            },
            verification: AssetVerification::BytesSha256,
        };
        self.insert(info, Input::Bytes(bytes))
    }

    /// Metadata is only a declaration. Actual input bytes are checked again by
    /// import/export; a reader must remain immutable throughout the operation.
    pub fn insert_reader(
        &mut self,
        info: AssetInfo,
        reader: &'a dyn ReaderAt,
    ) -> Result<(), Failure> {
        self.insert(info, Input::Reader(reader))
    }

    fn admit(&self, id: &AssetId, media_type: &str) -> Result<(), Failure> {
        if self.entries.contains_key(id) {
            return Err(Failure::new(
                FailureCode::ResourceConflict,
                "duplicate input identity",
            ));
        }
        if self.entries.len() >= 10_000 || media_type.is_empty() || media_type.len() > 255 {
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "input metadata limit",
            ));
        }
        Ok(())
    }

    fn insert(&mut self, info: AssetInfo, reader: Input<'a>) -> Result<(), Failure> {
        self.admit(&info.id, &info.descriptor.media_type)?;
        self.entries.insert(info.id.clone(), Entry { info, reader });
        Ok(())
    }
}
impl ExportAssets for Inputs<'_> {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, Failure> {
        let entry = self
            .entries
            .get(id)
            .ok_or_else(|| Failure::new(FailureCode::ResourceIncomplete, "input not provided"))?;
        Ok(ExportAsset {
            info: &entry.info,
            reader: &entry.reader,
        })
    }
}
