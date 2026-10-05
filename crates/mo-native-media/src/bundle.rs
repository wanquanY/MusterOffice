use crate::WorkerError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    process::Command,
};
const MAX_MANIFEST: u64 = 64 * 1024;
const MAX_ARTIFACT: u64 = 64 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    profile: String,
    os: String,
    arch: String,
    component_record_sha256: String,
    files: Vec<Artifact>,
    links: Vec<Link>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Artifact {
    path: String,
    byte_length: u64,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Link {
    path: String,
    target: String,
}
/// Operator-provided immutable bundle pinned by a trusted manifest digest.
/// Never derive that trust anchor from an untrusted bundle at runtime. Product
/// installation owns authenticating the release/channel and protecting storage.
#[derive(Clone, Debug)]
pub struct WorkerBundle {
    root: PathBuf,
    manifest_sha256: String,
}
impl WorkerBundle {
    pub fn open(root: PathBuf, expected_manifest_sha256: String) -> Result<Self, WorkerError> {
        if !hex_digest(&expected_manifest_sha256) {
            return Err(WorkerError::Invalid("manifest digest"));
        }
        let bundle = Self {
            root: root.canonicalize()?,
            manifest_sha256: expected_manifest_sha256,
        };
        bundle.verify()?;
        Ok(bundle)
    }
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
    pub(crate) fn command(&self) -> Result<Command, WorkerError> {
        self.verify()?;
        Ok(Command::new(self.root.join("bin/mo-media-worker")))
    }
    fn verify(&self) -> Result<(), WorkerError> {
        let path = self.root.join("manifest.json");
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(WorkerError::Invalid("manifest must be a regular file"));
        }
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_MANIFEST + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_MANIFEST
            || format!("{:x}", Sha256::digest(&bytes)) != self.manifest_sha256
        {
            return Err(WorkerError::Invalid("worker manifest changed or oversized"));
        }
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|e| WorkerError::Artifact(e.to_string()))?;
        if manifest.format != "musteroffice.media-worker-bundle/1"
            || manifest.profile != mo_media_wire::PROFILE
            || manifest.os != std::env::consts::OS
            || manifest.arch != std::env::consts::ARCH
            || !hex_digest(&manifest.component_record_sha256)
            || manifest.files.len() != 7
            || manifest.links.len() != 10
        {
            return Err(WorkerError::Invalid("worker bundle profile or inventory"));
        }
        for directory in ["bin", "lib"] {
            if !fs::symlink_metadata(self.root.join(directory))?.is_dir() {
                return Err(WorkerError::Invalid("worker directory type"));
            }
        }
        let mut files = BTreeSet::new();
        for artifact in manifest.files {
            if !path_shape(&artifact.path)
                || artifact.byte_length == 0
                || artifact.byte_length > MAX_ARTIFACT
                || !hex_digest(&artifact.sha256)
                || !files.insert(artifact.path.clone())
            {
                return Err(WorkerError::Invalid("worker artifact record"));
            }
            let path = self.root.join(&artifact.path);
            let stat = fs::symlink_metadata(&path)?;
            if !stat.is_file() || stat.len() != artifact.byte_length {
                return Err(WorkerError::Invalid("worker artifact length/type"));
            }
            let mut reader = File::open(path)?.take(artifact.byte_length + 1);
            let mut digest = Sha256::new();
            let mut length = 0u64;
            let mut block = [0u8; 65536];
            loop {
                let n = reader.read(&mut block)?;
                if n == 0 {
                    break;
                }
                length += n as u64;
                digest.update(&block[..n]);
            }
            if length != artifact.byte_length
                || format!("{:x}", digest.finalize()) != artifact.sha256
            {
                return Err(WorkerError::Invalid("worker artifact digest changed"));
            }
        }
        if !files.contains("bin/mo-media-worker")
            || files.iter().filter(|p| p.starts_with("bin/")).count() != 1
        {
            return Err(WorkerError::Invalid("worker executable inventory"));
        }
        let regular = files.clone();
        for link in manifest.links {
            if !path_shape(&link.path)
                || !link.path.starts_with("lib/")
                || !basename(&link.target)
                || !regular.contains(&format!("lib/{}", link.target))
                || !files.insert(link.path.clone())
            {
                return Err(WorkerError::Invalid("worker loader alias record"));
            }
            if fs::read_link(self.root.join(link.path))? != Path::new(&link.target) {
                return Err(WorkerError::Invalid("worker loader alias changed"));
            }
        }
        // Reject extra files and directory symlinks in the loader's search tree.
        for directory in ["bin", "lib"] {
            let path = self.root.join(directory);
            if !fs::symlink_metadata(&path)?.is_dir() {
                return Err(WorkerError::Invalid("worker directory type"));
            }
            let mut count = 0;
            for row in fs::read_dir(path)? {
                count += 1;
                if count > 32 {
                    return Err(WorkerError::Invalid("worker directory inventory"));
                }
                let row = row?;
                let name = row.file_name();
                let name = name
                    .to_str()
                    .ok_or(WorkerError::Invalid("worker artifact name"))?;
                if !files.remove(&format!("{directory}/{name}")) {
                    return Err(WorkerError::Invalid("unrecorded worker artifact"));
                }
            }
        }
        if !files.is_empty() {
            return Err(WorkerError::Invalid("missing worker artifact"));
        }
        Ok(())
    }
}
fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
}
fn basename(value: &str) -> bool {
    let mut parts = Path::new(value).components();
    matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none()
}
fn path_shape(value: &str) -> bool {
    let mut parts = Path::new(value).components();
    matches!(parts.next(), Some(Component::Normal(p)) if p == "bin" || p == "lib")
        && matches!(parts.next(), Some(Component::Normal(_)))
        && parts.next().is_none()
}
