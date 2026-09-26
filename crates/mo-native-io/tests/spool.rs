use mo_common::Digest;
use mo_native_io::{FileSpool, SpoolDirectory};
use mo_opc::{PackageBuilder, PackageLimits, PartName, ReaderAt, ResultSink, SealedOutput};
use sha2::{Digest as _, Sha256};
use std::{
    cell::Cell,
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mo-spool-test-{}-{epoch}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn entries(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect()
    }
    fn file(&self) -> PathBuf {
        let e = self.entries();
        assert_eq!(e.len(), 1);
        e[0].join("output")
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn execution_directory_removes_orphans_and_preserves_operator_siblings() {
    let root = Root::new();
    let marker = root.0.join("operator-owned");
    fs::write(&marker, b"keep").unwrap();
    for explicit in [false, true] {
        let directory = SpoolDirectory::create(&root.0).unwrap();
        let child = directory.path().join("interrupted-worker");
        fs::create_dir(&child).unwrap();
        fs::write(child.join("partial-output"), b"orphaned").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(directory.path()).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        if explicit {
            directory.discard().unwrap();
        } else {
            drop(directory);
        }
        assert_eq!(root.entries(), vec![marker.clone()]);
        assert_eq!(fs::read(&marker).unwrap(), b"keep");
    }
}

#[test]
fn private_spool_has_bounded_writes_sticky_failure_and_cleanup() {
    let root = Root::new();
    let mut spool = FileSpool::create(&root.0, 4).unwrap();
    spool.write_all(b"1234").unwrap();
    spool.flush().unwrap();
    assert_eq!(fs::read(root.file()).unwrap(), b"1234");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.file()).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(root.file().parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    assert!(spool.write_all(b"5").is_err());
    assert!(spool.write_all(b"x").is_err());
    assert!(spool.flush().is_err());
    assert!(spool.seal().is_err());
    assert!(root.entries().is_empty());
    let mut dropped = FileSpool::create(&root.0, 20).unwrap();
    dropped.write_all(b"still buffered").unwrap();
    drop(dropped);
    assert!(root.entries().is_empty());
}

#[test]
fn sealed_file_range_reads_and_explicit_cleanup_preserve_unrelated_files() {
    let root = Root::new();
    let mut spool = FileSpool::create(&root.0, 200_000).unwrap();
    let bytes: Vec<_> = (0..150_123).map(|i| (i % 251) as u8).collect();
    spool.write_all(&bytes).unwrap();
    let sealed = spool.seal().unwrap();
    assert_eq!(sealed.byte_length, bytes.len() as u64);
    assert_eq!(sealed.reader.byte_length(), sealed.byte_length);
    let mut buffer = vec![0; 70_001];
    sealed.reader.read_exact_at(&mut buffer, 65_530).unwrap();
    assert_eq!(buffer, bytes[65_530..135_531]);
    assert_eq!(sealed.reader.read_at(&mut buffer, u64::MAX).unwrap(), 0);
    assert_eq!(
        sealed
            .reader
            .read_at(&mut buffer, sealed.byte_length - 1)
            .unwrap(),
        1
    );
    let unrelated = root.file().parent().unwrap().join("unrelated");
    fs::write(&unrelated, b"retain").unwrap();
    assert!(sealed.reader.discard().is_err());
    assert_eq!(fs::read(unrelated).unwrap(), b"retain");
}

#[test]
fn actual_stored_pptx_streams_then_links_same_verified_file_without_overwrite() {
    let root = Root::new();
    let request = include_str!("../../../fixtures/presentations/native-export/request.json");
    let resources = include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
    let sink = FileSpool::create(&root.0, 1_000_000).unwrap();
    let verified = mo_kernel_api::export_pptx_json_to(
        request,
        &resources.as_slice(),
        resources.len() as u64,
        sink,
        &|| false,
    )
    .unwrap();
    let receipt = verified.receipt();
    assert_eq!(
        receipt.sha256.as_str(),
        // Matching-level defaults are part of the corrected native file bytes.
        "7e41967032bfbd9ca2b174d7183f3f5917339776105262a959eaf885af6c693e"
    );
    let source = root.file();
    assert_eq!(fs::metadata(&source).unwrap().len(), receipt.byte_length);
    let index = mo_pptx::source::inspect_source(
        verified.package(),
        mo_pptx::source::SourceLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(index.slides.len(), 2);
    let dest = root.0.join("native.pptx");
    verified.reader().link_new(&dest).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            fs::metadata(&source).unwrap().ino(),
            fs::metadata(&dest).unwrap().ino()
        );
    }
    assert!(verified.reader().link_new(&dest).is_err());
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&dest).unwrap())),
        receipt.sha256.as_str()
    );
    drop(verified);
    assert_eq!(root.entries(), vec![dest]);
}

struct Tamper {
    inner: FileSpool,
    path: PathBuf,
}
impl Write for Tamper {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.inner.write(b)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl ResultSink for Tamper {
    type Reader = mo_native_io::SealedFile;
    fn seal(self) -> io::Result<SealedOutput<Self::Reader>> {
        let sealed = self.inner.seal()?;
        let mut data = fs::read(&self.path)?;
        data[0] ^= 0xff;
        fs::write(self.path, data)?;
        Ok(sealed)
    }
}
fn tiny_package() -> PackageBuilder<'static> {
    let mut p = PackageBuilder::new();
    p.add_part(
        PartName::new("/document.xml").unwrap(),
        "application/xml".into(),
        b"<document/>".to_vec(),
    )
    .unwrap();
    p
}

#[test]
fn corrupted_real_stored_bytes_are_rejected_and_removed() {
    let root = Root::new();
    let inner = FileSpool::create(&root.0, 1_000_000).unwrap();
    let sink = Tamper {
        inner,
        path: root.file(),
    };
    assert!(
        tiny_package()
            .write_sealed(sink, PackageLimits::default(), &|| false)
            .is_err()
    );
    assert!(root.entries().is_empty());
}

#[test]
fn cancellation_and_output_limit_remove_unpublished_disk_files() {
    let root = Root::new();
    for limit in [0, 1, 200] {
        let sink = FileSpool::create(&root.0, limit).unwrap();
        assert!(
            tiny_package()
                .write_sealed(sink, PackageLimits::default(), &|| false)
                .is_err()
        );
        assert!(root.entries().is_empty());
    }
    for after in [0, 2, 6, 10, 15] {
        let count = Cell::new(0);
        let sink = FileSpool::create(&root.0, 1_000_000).unwrap();
        let cancelled = || {
            count.set(count.get() + 1);
            count.get() > after
        };
        assert!(
            tiny_package()
                .write_sealed(sink, PackageLimits::default(), &cancelled)
                .is_err()
        );
        assert!(root.entries().is_empty());
    }
}

#[test]
fn concurrent_spools_are_exclusive_and_publish_only_the_winning_link() {
    let root = Root::new();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let mut threads = Vec::new();
    for byte in 0u8..8 {
        let root = root.0.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            let mut sink = FileSpool::create(&root, 1).unwrap();
            sink.write_all(&[byte]).unwrap();
            let sealed = sink.seal().unwrap();
            barrier.wait();
            sealed.reader.link_new(&root.join("winner")).is_ok()
        }));
    }
    let wins = threads
        .into_iter()
        .map(|t| usize::from(t.join().unwrap()))
        .sum::<usize>();
    assert_eq!(wins, 1);
    assert_eq!(root.entries(), vec![root.0.join("winner")]);
    let bytes = fs::read(root.0.join("winner")).unwrap();
    assert_eq!(bytes.len(), 1);
    assert!(bytes[0] < 8);
}

/// Stateless deterministic incompressible resource; it never allocates its
/// complete input. This verifies the storage architecture, not image decoding.
struct Generated {
    length: u64,
    max_read: Cell<usize>,
}
fn block(index: u64) -> [u8; 8] {
    let mut x = index.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    (x ^ (x >> 31)).to_le_bytes()
}
impl ReaderAt for Generated {
    fn read_at(&self, b: &mut [u8], offset: u64) -> io::Result<usize> {
        let n = self.length.saturating_sub(offset).min(b.len() as u64) as usize;
        self.max_read.set(self.max_read.get().max(n));
        let mut copied = 0;
        while copied < n {
            let at = offset + copied as u64;
            let word = block(at / 8);
            let start = (at % 8) as usize;
            let size = (8 - start).min(n - copied);
            b[copied..copied + size].copy_from_slice(&word[start..start + size]);
            copied += size;
        }
        Ok(n)
    }
}

#[test]
fn multi_megabyte_input_and_output_use_bounded_io_without_whole_file_buffers() {
    let root = Root::new();
    let resource = Generated {
        length: 2 * 1024 * 1024 + 731,
        max_read: Cell::new(0),
    };
    let mut hash = Sha256::new();
    let mut offset = 0;
    let mut buffer = [0u8; 65_536];
    while offset < resource.length {
        let n = resource.read_at(&mut buffer, offset).unwrap();
        hash.update(&buffer[..n]);
        offset += n as u64;
    }
    let digest = Digest::from_sha256(hash.finalize().into());
    let mut package = tiny_package();
    let part = PartName::new("/media/generated.bin").unwrap();
    package
        .add_resource(
            part.clone(),
            "application/octet-stream".into(),
            &resource,
            resource.length,
            digest.clone(),
        )
        .unwrap();
    let verified = package
        .write_sealed(
            FileSpool::create(&root.0, 4 * 1024 * 1024).unwrap(),
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
    assert!(verified.receipt().byte_length > resource.length);
    assert_eq!(verified.package().parts()[&part].sha256, digest);
    assert_eq!(resource.max_read.get(), 65_536);
    assert_eq!(
        fs::metadata(root.file()).unwrap().len(),
        verified.receipt().byte_length
    );
    drop(verified);
    assert!(root.entries().is_empty());
}
