use mo_opc::{OpcError, Package, PackageBuilder, PackageLimits, PackageRead, PartName, ReaderAt};
use std::{cell::Cell, io};

// Borrowed, non-Clone host handle with short reads and an injected I/O fault.
// Immutable content and ownership stay with the host even after type erasure.
struct HostResource<'a> {
    bytes: &'a [u8],
    reads: Cell<usize>,
    fail: Cell<bool>,
}
impl ReaderAt for HostResource<'_> {
    fn read_at(&self, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
        self.reads.set(self.reads.get() + 1);
        if self.fail.get() {
            return Err(io::Error::other("injected retained resource read failure"));
        }
        let length = buffer.len().min(7);
        self.bytes.read_at(&mut buffer[..length], offset)
    }
}
fn fixture() -> (PartName, Vec<u8>) {
    let part = PartName::new("/opaque.bin").unwrap();
    let mut builder = PackageBuilder::new();
    builder
        .add_part(
            part.clone(),
            "application/octet-stream".into(),
            b"owned payload".as_slice(),
        )
        .unwrap();
    let bytes = builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap();
    (part, bytes)
}

#[test]
fn erased_read_retains_metadata_identity_limits_and_host_failure() {
    let (part, bytes) = fixture();
    let host = HostResource {
        bytes: &bytes,
        reads: Cell::new(0),
        fail: Cell::new(false),
    };
    let package = Package::open(&host, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let view: &dyn PackageRead = &package;
    let reads = host.reads.get();
    assert!(std::ptr::eq(view.parts(), package.parts()));
    assert!(std::ptr::eq(view.relationships(), package.relationships()));
    assert!(std::ptr::eq(view.content_types(), package.content_types()));
    assert!(std::ptr::eq(view.sha256(), package.sha256()));
    assert_eq!(view.byte_length(), bytes.len() as u64);
    assert!(!view.has_signatures());
    assert_eq!(host.reads.get(), reads);
    assert!(matches!(
        view.read_part(&part, 2, &|| false),
        Err(OpcError::Limit("part collection bytes"))
    ));
    assert!(matches!(
        view.read_part(&part, 32, &|| true),
        Err(OpcError::Cancelled)
    ));
    assert_eq!(host.reads.get(), reads);
    assert_eq!(
        view.read_part(&part, 32, &|| false).unwrap(),
        b"owned payload"
    );
    host.fail.set(true);
    assert!(
        view.read_part(&part, 32, &|| false)
            .unwrap_err()
            .to_string()
            .contains("injected retained resource read failure")
    );
    drop(package);
    host.fail.set(false);
    let mut first = [0; 4];
    host.read_exact_at(&mut first, 0).unwrap();
    assert_eq!(&first, b"PK\x03\x04");
}

#[test]
fn multiple_storage_types_share_the_same_read_contract() {
    let (part, bytes) = fixture();
    let owned = Package::open(
        bytes.clone(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let borrowed = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let reader: &dyn ReaderAt = &bytes;
    let erased = Package::open(
        reader,
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let packages: [&dyn PackageRead; 3] = [&owned, &borrowed, &erased];
    for package in packages {
        assert_eq!(package.parts(), owned.parts());
        assert_eq!(package.sha256(), owned.sha256());
        assert_eq!(
            package.read_part(&part, 32, &|| false).unwrap(),
            b"owned payload"
        );
        assert!(matches!(
            package.read_part(&PartName::new("/missing.bin").unwrap(), 32, &|| false),
            Err(OpcError::Structure(_))
        ));
    }
}
