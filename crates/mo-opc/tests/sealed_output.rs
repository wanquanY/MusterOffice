use mo_opc::*;
use std::{
    cell::Cell,
    io::{self, Write},
    rc::Rc,
};

fn package(text: &str) -> PackageBuilder<'static> {
    let mut p = PackageBuilder::new();
    p.add_part(
        PartName::new("/document.xml").unwrap(),
        "application/xml".into(),
        format!("<document>{text}</document>").into_bytes(),
    )
    .unwrap();
    p
}

#[derive(Clone, Copy, PartialEq)]
enum Fault {
    None,
    ShortWrite,
    ZeroWrite,
    Write,
    Flush,
    Seal,
    Length,
    Truncate,
    Read,
    Replace,
}

#[derive(Default)]
struct State {
    writes: Cell<usize>,
    reads: Cell<usize>,
    sealed: Cell<bool>,
    dropped_reader: Cell<bool>,
    abandoned: Cell<bool>,
}
struct Sink {
    bytes: Vec<u8>,
    fault: Fault,
    state: Rc<State>,
}
struct Reader {
    bytes: Vec<u8>,
    fault: Fault,
    state: Rc<State>,
}
impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.state.writes.set(self.state.writes.get() + 1);
        match self.fault {
            Fault::Write => Err(io::Error::other("write injection")),
            Fault::ZeroWrite => Ok(0),
            _ => {
                let n = if self.fault == Fault::ShortWrite {
                    b.len().min(7)
                } else {
                    b.len()
                };
                self.bytes.extend_from_slice(&b[..n]);
                Ok(n)
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.fault == Fault::Flush {
            Err(io::Error::other("flush injection"))
        } else {
            Ok(())
        }
    }
}
impl ResultSink for Sink {
    type Reader = Reader;
    fn seal(mut self) -> io::Result<SealedOutput<Reader>> {
        if self.fault == Fault::Seal {
            return Err(io::Error::other("seal injection"));
        }
        self.state.sealed.set(true);
        let mut bytes = std::mem::take(&mut self.bytes);
        match self.fault {
            Fault::Truncate => {
                bytes.pop();
            }
            Fault::Replace => {
                let replacement = package("omega")
                    .to_bytes(PackageLimits::default(), &|| false)
                    .unwrap();
                assert_eq!(replacement.len(), bytes.len());
                bytes = replacement;
            }
            _ => (),
        }
        let length = bytes.len() as u64 + u64::from(self.fault == Fault::Length);
        Ok(SealedOutput {
            reader: Reader {
                bytes,
                fault: self.fault,
                state: self.state.clone(),
            },
            byte_length: length,
        })
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        if !self.state.sealed.get() {
            self.state.abandoned.set(true);
        }
    }
}
impl ReaderAt for Reader {
    fn read_at(&self, b: &mut [u8], offset: u64) -> io::Result<usize> {
        self.state.reads.set(self.state.reads.get() + 1);
        if self.fault == Fault::Read {
            return Err(io::Error::other("read injection"));
        }
        let n = b.len().min(11);
        self.bytes.read_at(&mut b[..n], offset)
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        self.state.dropped_reader.set(true);
    }
}
fn sink(fault: Fault) -> (Sink, Rc<State>) {
    let state = Rc::new(State::default());
    (
        Sink {
            bytes: Vec::new(),
            fault,
            state: state.clone(),
        },
        state,
    )
}

#[test]
fn sealed_package_reopens_the_host_reader_and_accepts_short_io() {
    let baseline = package("alpha")
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap();
    let (sink, state) = sink(Fault::ShortWrite);
    let verified = package("alpha")
        .write_sealed(sink, PackageLimits::default(), &|| false)
        .unwrap();
    assert!(state.sealed.get() && state.reads.get() > 20 && state.writes.get() > 20);
    assert_eq!(verified.reader().bytes, baseline);
    assert_eq!(verified.receipt().byte_length, baseline.len() as u64);
    assert_eq!(&verified.receipt().sha256, verified.package().sha256());
    assert!(!state.dropped_reader.get());
    drop(verified);
    assert!(state.dropped_reader.get());
}

#[test]
fn writer_storage_and_readback_failures_never_return_verified_outputs() {
    for fault in [
        Fault::ZeroWrite,
        Fault::Write,
        Fault::Flush,
        Fault::Seal,
        Fault::Length,
        Fault::Truncate,
        Fault::Read,
    ] {
        let (sink, state) = sink(fault);
        assert!(
            package("alpha")
                .write_sealed(sink, PackageLimits::default(), &|| false)
                .is_err()
        );
        assert!(state.abandoned.get() || state.dropped_reader.get());
    }
}

#[test]
fn valid_but_different_stored_package_is_rejected_by_writer_digest() {
    let (sink, state) = sink(Fault::Replace);
    let error = package("alpha")
        .write_sealed(sink, PackageLimits::default(), &|| false)
        .err()
        .unwrap();
    assert!(matches!(error,OpcError::Preservation(message) if message.contains("digest differs")));
    assert!(state.reads.get() > 0 && state.dropped_reader.get());
}

#[test]
fn cancellation_during_write_seal_and_actual_readback_abandons_output() {
    for phase in 0..4 {
        let (sink, state) = sink(Fault::None);
        let check = || match phase {
            0 => true,
            1 => state.writes.get() > 2,
            2 => state.sealed.get(),
            _ => state.reads.get() > 10,
        };
        assert!(matches!(
            package("alpha").write_sealed(sink, PackageLimits::default(), &check),
            Err(OpcError::Cancelled)
        ));
        assert!(state.abandoned.get() || state.dropped_reader.get());
    }
}

#[test]
fn preservation_rewrites_share_the_same_sealed_byte_verifier() {
    let source = package("alpha")
        .write_sealed(Vec::new(), PackageLimits::default(), &|| false)
        .unwrap();
    let mut plan = RewritePlan::new();
    let (sink, _) = sink(Fault::ShortWrite);
    let same = plan
        .write_sealed(source.package(), sink, &|| false)
        .unwrap();
    assert_eq!(same.reader().bytes, *source.reader());
    plan.replace_part(
        PartName::new("/document.xml").unwrap(),
        b"<updated/>".to_vec(),
    )
    .unwrap();
    let (destination, state) = self::sink(Fault::None);
    let updated = plan
        .write_sealed(source.package(), destination, &|| false)
        .unwrap();
    assert_eq!(
        updated
            .package()
            .read_part(&PartName::new("/document.xml").unwrap(), 100, &|| false)
            .unwrap(),
        b"<updated/>"
    );
    assert!(state.reads.get() > 0);
    assert_ne!(updated.receipt().sha256, source.receipt().sha256);
}
