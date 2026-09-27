use mo_embedded_sdk::{NativeExporter, common::Digest};
use mo_native_compute::workspace::Directory;
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    input_directory: PathBuf,
    output_directory: PathBuf,
    temporary_directory: PathBuf,
    export_worker: Option<Worker>,
    #[serde(default = "two")]
    computation_slots: usize,
    #[serde(default = "two")]
    control_slots: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Worker {
    path: PathBuf,
    sha256: Digest,
}
fn two() -> usize {
    2
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PackagedRuntime {
    format: String,
    target_os: String,
    target_arch: String,
    worker_sha256: Digest,
}

pub(super) struct Files {
    pub inputs: Directory,
    pub outputs: Directory,
    pub temporary: Directory,
    pub exporter: Option<NativeExporter>,
}

impl Config {
    /// Bind caller-owned file configuration to this package's matching worker.
    /// No directories, documents or persistent configuration are created.
    pub fn from_package(package: &Path, caller: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !package.is_absolute() || !caller.is_absolute() {
            return Err("package and caller configuration paths must be absolute".into());
        }
        let parent = package.parent().ok_or("package root missing")?;
        let root = Directory::open(parent)?;
        let name = package
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("package filename must be UTF-8")?;
        let mut text = String::new();
        root.read(name)?.take(65537).read_to_string(&mut text)?;
        if text.len() > 65536 {
            return Err("package runtime manifest byte limit".into());
        }
        let runtime: PackagedRuntime = mo_embedded_sdk::common::from_json_str(&text)?;
        if runtime.format != "musteroffice.mcp-package/1-draft"
            || runtime.target_os != std::env::consts::OS
            || runtime.target_arch != std::env::consts::ARCH
        {
            return Err("unsupported package format or target platform".into());
        }
        let mut config = Self::read(caller)?;
        if config.export_worker.is_some() {
            return Err(
                "packaged MCP selects its matching worker; caller config must omit exportWorker"
                    .into(),
            );
        }
        let bin = Directory::open(&root.child("bin")?)?;
        let filename = if cfg!(windows) {
            "mo-export-worker.exe"
        } else {
            "mo-export-worker"
        };
        // A packaged executable is a regular file, never a redirected entry.
        // The host keeps the package tree stable while this process is active.
        drop(bin.read(filename)?);
        // NativeExporter verifies the bytes against this digest before launching;
        // no ambient PATH lookup or caller-controlled worker is substituted.
        config.export_worker = Some(Worker {
            path: bin.child(filename)?,
            sha256: runtime.worker_sha256,
        });
        Ok(config)
    }
    pub fn computation_slots(&self) -> usize {
        self.computation_slots
    }
    pub fn control_slots(&self) -> usize {
        self.control_slots
    }
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut text = String::new();
        File::open(path)?.take(65537).read_to_string(&mut text)?;
        if text.len() > 65536 {
            return Err("MCP configuration byte limit".into());
        }
        let config: Self = mo_embedded_sdk::common::from_json_str(&text)?;
        if [
            &config.input_directory,
            &config.output_directory,
            &config.temporary_directory,
        ]
        .iter()
        .any(|p| !p.is_absolute())
            || config
                .export_worker
                .as_ref()
                .is_some_and(|w| !w.path.is_absolute())
            || !(1..=8).contains(&config.computation_slots)
            || !(1..=8).contains(&config.control_slots)
        {
            return Err("MCP configuration paths or concurrency".into());
        }
        Ok(config)
    }
    pub(super) fn open(&self) -> Result<Files, Box<dyn std::error::Error>> {
        let inputs = Directory::open(&self.input_directory)?;
        let outputs = Directory::open(&self.output_directory)?;
        let temporary = Directory::open(&self.temporary_directory)?;
        // Temporary recovery must never share a directory with caller inputs
        // or final files; no role is inferred from a request argument.
        if temporary.path().starts_with(inputs.path())
            || inputs.path().starts_with(temporary.path())
            || temporary.path().starts_with(outputs.path())
            || outputs.path().starts_with(temporary.path())
        {
            return Err("temporary directory must be separate from input/output trees".into());
        }
        let exporter = self
            .export_worker
            .as_ref()
            .map(|w| {
                NativeExporter::new(
                    w.path.clone(),
                    w.sha256.clone(),
                    temporary.path().into(),
                    Duration::from_secs(60),
                )
            })
            .transpose()?;
        Ok(Files {
            inputs,
            outputs,
            temporary,
            exporter,
        })
    }
}
