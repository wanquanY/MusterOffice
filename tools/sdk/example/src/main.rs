//! Local SDK consumer. Paths come from the operator, not an Agent request.
use mo_embedded_sdk::{
    NativeExporter,
    common::{Digest, RequestId},
    delivery::{self, Content, DeliveryAsset, DeliveryError, DeliverySource},
    edit::SnapshotRecord,
    native_io::{FileSpool, SealedFile},
    opc::{ReaderAt, ResultSink},
    operation::{
        AssetId, AssetInfo, ExportAsset, ExportAssets, Failure, FailureCode, OperationRequest,
    },
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};

struct Inputs(BTreeMap<AssetId, (AssetInfo, SealedFile)>);
impl ExportAssets for Inputs {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, Failure> {
        let (info, reader) = self.0.get(id).ok_or_else(|| {
            Failure::new(FailureCode::NotAuthorized, "asset not provided by host")
        })?;
        Ok(ExportAsset { info, reader })
    }
}
struct StoredFile {
    file: Mutex<File>,
    length: u64,
}
impl ReaderAt for StoredFile {
    fn read_at(&self, output: &mut [u8], offset: u64) -> io::Result<usize> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("storage lock"))?;
        file.seek(SeekFrom::Start(offset))?;
        file.read(output)
    }
}
struct Stored(BTreeMap<RequestId, StoredFile>);
impl DeliverySource for Stored {
    fn open(&self, id: &RequestId) -> Result<Content<'_>, DeliveryError> {
        let file = self
            .0
            .get(id)
            .ok_or(DeliveryError::Invalid("stored asset missing"))?;
        Ok(Content {
            reader: file,
            byte_length: file.length,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("expected: input-directory new-output-directory worker worker-sha256".into());
    }
    let input = PathBuf::from(&args[0]);
    let root = PathBuf::from(&args[1]);
    fs::create_dir(&root)?;
    let spool = root.join("spool");
    fs::create_dir(&spool)?;
    let request: OperationRequest =
        mo_embedded_sdk::common::from_json_str(&fs::read_to_string(input.join("request.json"))?)?;
    let snapshot: SnapshotRecord =
        mo_embedded_sdk::common::from_json_str(&fs::read_to_string(input.join("snapshot.json"))?)?;
    let index: serde_json::Value = serde_json::from_slice(&fs::read(input.join("assets.json"))?)?;
    let mut assets = Inputs(BTreeMap::new());
    for item in index.as_array().ok_or("asset list")? {
        let info: AssetInfo = serde_json::from_value(item["info"].clone())?;
        let name = item["file"].as_str().ok_or("asset file")?;
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err("asset file must be a leaf".into());
        }
        let source = File::open(input.join(name))?;
        if source.metadata()?.len() > 128 * 1024 * 1024 {
            return Err("input asset limit".into());
        }
        let mut sink = FileSpool::create(&spool, 128 * 1024 * 1024)?;
        io::copy(&mut source.take(128 * 1024 * 1024 + 1), &mut sink)?;
        let sealed = sink.seal()?;
        if sealed.byte_length != info.descriptor.byte_length.get() {
            return Err("asset length differs".into());
        }
        if assets
            .0
            .insert(info.id.clone(), (info, sealed.reader))
            .is_some()
        {
            return Err("duplicate input asset".into());
        }
    }
    let exporter = NativeExporter::new(
        args[2].clone().into(),
        Digest::try_from(args[3].clone())?,
        spool.clone(),
        Duration::from_secs(60),
    )?;
    let candidate = exporter.prepare(&request, snapshot, &assets, &|| false)?;
    let mut stored = Stored(BTreeMap::new());
    let mut files = Vec::new();
    for (i, asset) in candidate.assets().iter().enumerate() {
        let source = candidate.open(&asset.id)?;
        let extension = match asset.role {
            delivery::AssetRole::Pptx => "pptx",
            delivery::AssetRole::Preview => "png",
            _ => "bin",
        };
        let name = format!("{i:03}.{extension}");
        let path = root.join(&name);
        let mut output = File::create_new(&path)?;
        let mut offset = 0;
        let mut buffer = [0; 65536];
        while offset < source.byte_length {
            let n = (source.byte_length - offset).min(buffer.len() as u64) as usize;
            source.reader.read_exact_at(&mut buffer[..n], offset)?;
            output.write_all(&buffer[..n])?;
            offset += n as u64;
        }
        output.sync_all()?;
        drop(output);
        let file = File::open(path)?;
        stored.0.insert(
            asset.id.clone(),
            StoredFile {
                length: file.metadata()?.len(),
                file: Mutex::new(file),
            },
        );
        files.push(serde_json::json!({"file":name,"asset":asset}));
    }
    // Inspect final copied bytes, not a serialized "verified" assertion.
    let verified = delivery::inspect(
        &candidate.receipt().bundle,
        candidate.expectation(),
        &stored,
        Default::default(),
        &|| false,
    )?;
    let bundle = serde_json::to_vec(candidate.receipt())?;
    let returned_assets: Vec<DeliveryAsset> = candidate.assets().to_vec();
    fs::write(root.join("receipt.json"), &bundle)?;
    fs::write(root.join("files.json"), serde_json::to_vec_pretty(&files)?)?;
    fs::write(
        root.join("inspection.json"),
        serde_json::to_vec_pretty(verified.report())?,
    )?;
    candidate.discard()?;
    drop(assets);
    if fs::read_dir(spool)?.next().is_some() {
        return Err("private spool cleanup incomplete".into());
    }
    println!(
        "{}",
        serde_json::json!({"assets":returned_assets.len(),"pages":verified.report().pages,"committed":false})
    );
    Ok(())
}
