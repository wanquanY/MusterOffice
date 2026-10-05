#![cfg(unix)]
mod support;
use mo_native_media::{NativeContainer, WorkerBundle};
use std::{
    cell::Cell,
    fs,
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
struct CopyBundle(PathBuf);
impl CopyBundle {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let source =
            PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").expect("explicit bundle"));
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "mo-media-bundle-{}-{suffix}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for dir in ["bin", "lib"] {
            fs::create_dir(root.join(dir)).unwrap();
            for row in fs::read_dir(source.join(dir)).unwrap() {
                let row = row.unwrap();
                let dest = root.join(dir).join(row.file_name());
                if row.file_type().unwrap().is_symlink() {
                    symlink(fs::read_link(row.path()).unwrap(), dest).unwrap();
                } else {
                    fs::copy(row.path(), dest).unwrap();
                }
            }
        }
        fs::copy(source.join("manifest.json"), root.join("manifest.json")).unwrap();
        Self(root)
    }
    fn open(&self) -> Result<WorkerBundle, mo_native_media::WorkerError> {
        WorkerBundle::open(
            self.0.clone(),
            std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").unwrap(),
        )
    }
    fn library(&self) -> PathBuf {
        fs::read_dir(self.0.join("lib"))
            .unwrap()
            .map(Result::unwrap)
            .find(|r| r.file_type().unwrap().is_file())
            .unwrap()
            .path()
    }
}
impl Drop for CopyBundle {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
#[ignore = "requires an explicitly built bundle"]
fn changed_library_is_rejected_again_at_launch_before_source_access() {
    let copy = CopyBundle::new();
    let bundle = copy.open().unwrap();
    let path = copy.library();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 1;
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&byte).unwrap();
    file.sync_all().unwrap();
    let accessed = Cell::new(false);
    assert!(
        NativeContainer::open(
            &bundle,
            |_: u64, _: &mut [u8]| {
                accessed.set(true);
                Ok(0)
            },
            100,
            support::options(),
            &|| false
        )
        .is_err()
    );
    assert!(!accessed.get());
}
#[test]
#[ignore = "requires an explicitly built bundle"]
fn loader_alias_extra_file_and_directory_links_are_rejected() {
    let copy = CopyBundle::new();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(copy.0.join("manifest.json")).unwrap()).unwrap();
    let link = &manifest["links"][0];
    let path = copy.0.join(link["path"].as_str().unwrap());
    let target = link["target"].as_str().unwrap();
    fs::remove_file(&path).unwrap();
    symlink("missing.dylib", &path).unwrap();
    assert!(copy.open().is_err());
    fs::remove_file(&path).unwrap();
    symlink(target, &path).unwrap();
    assert!(copy.open().is_ok());
    let extra = copy.0.join("lib/unrecorded");
    fs::write(&extra, b"extra").unwrap();
    assert!(copy.open().is_err());
    fs::remove_file(extra).unwrap();
    let moved = copy.0.join("lib-original");
    fs::rename(copy.0.join("lib"), &moved).unwrap();
    symlink(Path::new("lib-original"), copy.0.join("lib")).unwrap();
    assert!(copy.open().is_err());
}
#[test]
#[ignore = "requires an explicitly built bundle"]
fn manifest_trust_anchor_and_platform_are_not_inferred_from_bundle() {
    let copy = CopyBundle::new();
    assert!(WorkerBundle::open(copy.0.clone(), "0".repeat(64)).is_err());
    let path = copy.0.join("manifest.json");
    let mut record: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    record["arch"] = "other-architecture".into();
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    assert!(copy.open().is_err());
    use sha2::{Digest, Sha256};
    let deliberately_reapproved_pin = format!("{:x}", Sha256::digest(fs::read(path).unwrap()));
    assert!(WorkerBundle::open(copy.0.clone(), deliberately_reapproved_pin).is_err());
}
