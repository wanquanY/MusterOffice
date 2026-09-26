use crate::{OpcError, Package, PackageLimits, ReaderAt, WriteReceipt, check_cancel};
use std::io::{self, Write};

/// Injected host staging capability. Writers may leave partial bytes on failure;
/// dropping the sink must abandon them, never publish a successful result.
///
/// `seal` flushes pending storage, freezes mutation and returns a reader over the
/// actual stored bytes. Its length is measured by the host after writing. The
/// host keeps those bytes immutable/alive until the reader is dropped. Neither
/// a write receipt nor sealing is a product commit or a quality claim.
pub trait ResultSink: Write {
    type Reader: ReaderAt;

    fn seal(self) -> io::Result<SealedOutput<Self::Reader>>;
}

/// Host assertion of an immutable, range-readable resource. It is intentionally
/// distinct from `VerifiedPackage`: successful storage is not package validation.
pub struct SealedOutput<R> {
    pub reader: R,
    pub byte_length: u64,
}

/// Actual sealed bytes passed OPC graph, ZIP/CRC, XML and digest verification.
/// This does not prove PresentationML XSD, layout, editability or target-app
/// compatibility. Product publication still belongs to the authorized host.
pub struct VerifiedPackage<R> {
    package: Package<R>,
    receipt: WriteReceipt,
}

impl<R: ReaderAt> VerifiedPackage<R> {
    pub fn package(&self) -> &Package<R> {
        &self.package
    }

    pub fn receipt(&self) -> &WriteReceipt {
        &self.receipt
    }

    pub fn reader(&self) -> &R {
        self.package.archive.get_ref()
    }

    /// Transfers ownership of the same sealed resource without collecting its
    /// bytes. The receiver must preserve immutability and digest-bound evidence.
    pub fn into_reader(self) -> R {
        self.package.archive.into_inner()
    }
}

/// Shared completion path for authored packages and source-preserving rewrites.
/// Checks the sink's actual bytes, not an in-memory copy retained by the writer.
pub(crate) fn seal_and_verify<S: ResultSink>(
    mut sink: S,
    receipt: WriteReceipt,
    limits: PackageLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<VerifiedPackage<S::Reader>, OpcError> {
    check_cancel(cancelled)?;
    sink.flush()?;
    check_cancel(cancelled)?;
    let sealed = sink.seal()?;
    check_cancel(cancelled)?;
    if sealed.byte_length != receipt.byte_length {
        return Err(OpcError::Preservation(
            "sealed output length differs from writer receipt".into(),
        ));
    }
    let package = Package::open(sealed.reader, sealed.byte_length, limits, cancelled)?;
    if package.sha256() != &receipt.sha256 {
        return Err(OpcError::Preservation(
            "sealed output digest differs from writer receipt".into(),
        ));
    }
    check_cancel(cancelled)?;
    Ok(VerifiedPackage { package, receipt })
}

/// Convenience storage for callers explicitly choosing bounded memory output.
/// Package byte limits are enforced by the Writer before each write.
impl ResultSink for Vec<u8> {
    type Reader = Vec<u8>;

    fn seal(self) -> io::Result<SealedOutput<Self::Reader>> {
        Ok(SealedOutput {
            byte_length: self.len() as u64,
            reader: self,
        })
    }
}
