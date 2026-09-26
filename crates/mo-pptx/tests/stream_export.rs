use mo_opc::{OpcError, ReaderAt, ResultSink, SealedOutput};
use mo_pptx::{PptxError, PptxLimits, export, export_to};
use std::{
    cell::Cell,
    io::{self, Write},
    rc::Rc,
};
mod support;

struct Sink {
    bytes: Vec<u8>,
    bytes_written: Rc<Cell<usize>>,
    reads: Rc<Cell<usize>>,
}
struct Sealed {
    bytes: Vec<u8>,
    reads: Rc<Cell<usize>>,
}
impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = b.len().min(19);
        self.bytes.extend_from_slice(&b[..n]);
        self.bytes_written.set(self.bytes.len());
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl ResultSink for Sink {
    type Reader = Sealed;
    fn seal(self) -> io::Result<SealedOutput<Sealed>> {
        Ok(SealedOutput {
            byte_length: self.bytes.len() as u64,
            reader: Sealed {
                bytes: self.bytes,
                reads: self.reads,
            },
        })
    }
}
impl ReaderAt for Sealed {
    fn read_at(&self, b: &mut [u8], offset: u64) -> io::Result<usize> {
        self.reads.set(self.reads.get() + 1);
        self.bytes.read_at(b, offset)
    }
}
fn sink() -> (Sink, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let writes = Rc::new(Cell::new(0));
    let reads = Rc::new(Cell::new(0));
    (
        Sink {
            bytes: Vec::new(),
            bytes_written: writes.clone(),
            reads: reads.clone(),
        },
        writes,
        reads,
    )
}

#[test]
fn pptx_stream_and_memory_paths_share_native_bytes_and_sealed_readback() {
    let (document, defaults) = support::input();
    let expected = export(
        &document,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let (sink, writes, reads) = sink();
    let output = export_to(
        &document,
        &defaults,
        &support::resources(),
        sink,
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(output.reader().bytes, expected);
    assert_eq!(writes.get(), expected.len());
    assert!(reads.get() > 20);
    let index = mo_pptx::source::inspect_source(
        output.package(),
        mo_pptx::source::SourceLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(index.slides.len(), 2);
    assert_eq!(
        index
            .surfaces
            .values()
            .map(|s| s.objects.len())
            .sum::<usize>(),
        15
    );
    assert_eq!(
        output.receipt().sha256.as_str(),
        // Matching-level styles replace ignored defPPr in newly authored files.
        "7e41967032bfbd9ca2b174d7183f3f5917339776105262a959eaf885af6c693e"
    );
}

#[test]
fn pptx_stream_rejects_invalid_input_before_write_and_cancels_actual_output() {
    let (mut document, defaults) = support::input();
    document.slide_order.push(document.slide_order[0].clone());
    let (sink, writes, _) = sink();
    assert!(matches!(
        export_to(
            &document,
            &defaults,
            &support::resources(),
            sink,
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::InvalidDocument(_))
    ));
    assert_eq!(writes.get(), 0);
    let (document, defaults) = support::input();
    for readback in [false, true] {
        let (sink, writes, reads) = self::sink();
        let check = || {
            if readback {
                reads.get() > 10
            } else {
                writes.get() > 500
            }
        };
        assert!(matches!(
            export_to(
                &document,
                &defaults,
                &support::resources(),
                sink,
                PptxLimits::default(),
                &check
            ),
            Err(PptxError::Opc(OpcError::Cancelled))
        ));
    }
}
