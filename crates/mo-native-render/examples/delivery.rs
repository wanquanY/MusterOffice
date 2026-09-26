//! Actual private delivery calculation with a native worker. Diagnostic files
//! are verification outputs, not public assets or a successful host export job.
use mo_common::{Digest, ResourceId};
use mo_native_io::FileSpool;
use mo_native_render::NativePreviewRenderer;
use mo_pptx::{PptxError, ResourceData, Resources};
use mo_presentation_delivery::*;
use mo_presentation_edit::Snapshot;
use serde::Deserialize;
use std::{fs, path::PathBuf, time::Duration};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    document: mo_presentation_model::Document,
    settings: DeliverySettings,
}
struct Store(PathBuf);
impl OutputStore for Store {
    type Sink = FileSpool;
    fn create(&mut self, _: &str, _: &str, limit: u64) -> Result<FileSpool, DeliveryError> {
        Ok(FileSpool::create(&self.0, limit)?)
    }
}
struct Images(Vec<u8>);
impl Resources for Images {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        if id.as_str() != "resource:checker" {
            return Err(PptxError::ResourceRequired(id.clone()));
        }
        Ok(ResourceData {
            reader: &self.0,
            byte_length: self.0.len() as u64,
        })
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err(
            "expected: output-root input-json resources-bin fonts-bin worker worker-sha256".into(),
        );
    }
    let root = PathBuf::from(&args[0]);
    fs::create_dir(&root)?;
    let spool = root.join("spool");
    fs::create_dir(&spool)?;
    let input: Input = mo_common::from_json_str(&fs::read_to_string(&args[1])?)?;
    let images = Images(fs::read(&args[2])?);
    let fonts = fs::read(&args[3])?;
    let mut renderer = NativePreviewRenderer::new(
        PathBuf::from(&args[4]),
        serde_json::from_value::<Digest>(serde_json::Value::String(args[5].clone()))?,
        Duration::from_secs(60),
    )?;
    let snapshot = Snapshot::new(input.document, Default::default())?.into_record();
    let result = build(
        DeliveryInputs {
            snapshot,
            settings: &input.settings,
            resources: &images,
            fonts: Content {
                reader: &fonts,
                byte_length: fonts.len() as u64,
            },
        },
        &mut Store(spool.clone()),
        &mut renderer,
        DeliveryLimits::default(),
        &|| false,
    )?;
    let mut files = Vec::new();
    for (i, artifact) in result.artifacts().iter().enumerate() {
        let extension = match artifact.asset().role {
            AssetRole::Pptx => "pptx",
            AssetRole::Preview | AssetRole::Image => "png",
            _ if artifact.asset().media_type.ends_with("json") => "json",
            _ => "bin",
        };
        let name = format!("{i:03}.{extension}");
        artifact.reader().link_new(&root.join(&name))?;
        files
            .push(serde_json::json!({"name":artifact.name(),"asset":artifact.asset(),"file":name}));
    }
    fs::write(
        root.join("bundle.json"),
        serde_json::to_vec_pretty(result.bundle())?,
    )?;
    fs::write(root.join("files.json"), serde_json::to_vec_pretty(&files)?)?;
    println!(
        "{}",
        serde_json::json!({"artifacts":files.len(),"pages":result.bundle().previews.len(),"semanticDigest":result.semantic_digest(),"settingsDigest":result.settings_digest(),"publicExport":false})
    );
    drop(result);
    if fs::read_dir(&spool)?.next().is_some() {
        return Err("private spool cleanup incomplete".into());
    }
    Ok(())
}
