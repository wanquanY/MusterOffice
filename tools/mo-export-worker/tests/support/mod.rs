use mo_common::{ByteLength, Digest, RequestId, ResourceId};
use mo_native_export::NativeExporter;
use mo_opc::ReaderAt;
use mo_operation_service::*;
use mo_presentation_delivery::RendererIdentity;
use mo_presentation_edit::{Snapshot, SnapshotRecord};
use sha2::{Digest as _, Sha256};
use std::{
    cell::Cell,
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Root(PathBuf);
// Test-only lifetime guard: the production execution-directory implementation
// is deliberately not used to prove its own cleanup assertion.
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
impl Root {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mo-embedded-test-{}-{epoch}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("spool")).unwrap();
        fs::write(path.join("host-owned"), b"keep").unwrap();
        Self(path)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn spool(&self) -> PathBuf {
        self.path().join("spool")
    }
    pub fn clean(&self) {
        let entries: Vec<_> = fs::read_dir(self.spool())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, [".mo-executions-v1"]);
        let entries: Vec<_> = fs::read_dir(self.spool().join(".mo-executions-v1"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, ["registry.lock"]);
        assert_eq!(fs::read(self.path().join("host-owned")).unwrap(), b"keep");
    }
    pub fn executions(&self) -> Vec<PathBuf> {
        fs::read_dir(self.spool().join(".mo-executions-v1"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.file_name().unwrap() != "registry.lock")
            .collect()
    }
}
pub fn digest(bytes: &[u8]) -> Digest {
    Digest::from_sha256(Sha256::digest(bytes).into())
}
pub struct Bytes {
    pub bytes: Vec<u8>,
    pub max_read: Cell<usize>,
    pub fail: bool,
}
impl ReaderAt for Bytes {
    fn read_at(&self, output: &mut [u8], offset: u64) -> io::Result<usize> {
        self.max_read.set(self.max_read.get().max(output.len()));
        if self.fail {
            return Err(io::Error::other("injected authorized source failure"));
        }
        self.bytes.read_at(output, offset)
    }
}
pub struct Assets(pub Vec<(AssetInfo, Bytes)>);
impl ExportAssets for Assets {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, Failure> {
        let (info, reader) = self
            .0
            .iter()
            .find(|(info, _)| &info.id == id)
            .ok_or_else(|| Failure::new(FailureCode::NotAuthorized, "asset not authorized"))?;
        Ok(ExportAsset { info, reader })
    }
}
pub fn input(renderer: RendererIdentity) -> (SnapshotRecord, OperationRequest, Assets) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    let snapshot = Snapshot::new(
        serde_json::from_value(value["document"].clone()).unwrap(),
        Default::default(),
    )
    .unwrap()
    .into_record();
    let mut assets = Assets(vec![]);
    for (id, media, bytes) in [
        (
            "image:1",
            "image/png",
            include_bytes!("../../../../fixtures/presentations/native-export/resources.bin")
                .as_slice(),
        ),
        (
            "font:1",
            "font/ttf",
            include_bytes!("../../../../fixtures/fonts/owned.ttf").as_slice(),
        ),
    ] {
        assets.0.push((
            AssetInfo {
                id: AssetId::new(id).unwrap(),
                descriptor: AssetDescriptor {
                    sha256: digest(bytes),
                    byte_length: ByteLength::new(bytes.len() as u64),
                    media_type: media.into(),
                },
                verification: AssetVerification::BytesSha256,
            },
            Bytes {
                bytes: bytes.to_vec(),
                max_read: Cell::new(0),
                fail: false,
            },
        ));
    }
    let request = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("export:embedded-test").unwrap(),
        profile_id: OperationProfile::ResourceDelivery,
        output_mode: OutputMode::Job,
        action: DocumentAction::Export {
            document_id: snapshot.document.id.clone(),
            base_revision: snapshot.revision.clone(),
            settings: Box::new(ExportSettings {
                delivery: serde_json::from_value(value["settings"].clone()).unwrap(),
                resources: vec![AssetBinding {
                    resource_id: ResourceId::new("resource:checker").unwrap(),
                    asset_id: assets.0[0].0.id.clone(),
                }],
                font_asset_id: Some(assets.0[1].0.id.clone()),
                renderer,
            }),
        },
    };
    (snapshot, request, assets)
}
pub fn worker_path() -> PathBuf {
    std::env::var_os("MO_EXPORT_WORKER_TEST_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_mo-export-worker")))
}
pub fn exporter(root: &Root) -> NativeExporter {
    let exe = worker_path();
    let sha = mo_native_export::executable_digest(&exe).unwrap();
    if std::env::var_os("MO_EXPORT_WORKER_TEST_BIN").is_some() {
        assert_eq!(
            sha.as_str(),
            std::env::var("MO_EXPORT_WORKER_TEST_SHA256").expect("explicit digest")
        );
    }
    NativeExporter::new(exe, sha, root.spool(), Duration::from_secs(60)).unwrap()
}
