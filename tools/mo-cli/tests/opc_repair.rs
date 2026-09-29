use serde_json::Value;
use std::{fs, io::Write, path::PathBuf, process::Command};

struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn explicit_cli_repair_preserves_source_and_publishes_only_valid_new_output() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        Root(std::env::temp_dir().join(format!("mo-opc-repair-{}-{stamp}", std::process::id())));
    fs::create_dir(&root.0).unwrap();
    let mut zip = rawzip::ZipArchiveWriter::new(Vec::new());
    for (name,bytes) in [("[Content_Types].xml",b"<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Default Extension='xml' ContentType='application/xml'/><Override PartName='/missing.xml' ContentType='application/xml'/></Types>".as_slice()),("doc.xml",b"<owned>kept</owned>".as_slice())] {
        let (entry,config)=zip.new_file(rawzip::path::EntryPath::verbatim(name.as_bytes())).start().unwrap();
        let mut stream=config.wrap(entry);stream.write_all(bytes).unwrap();
        let (entry,descriptor)=stream.finish().unwrap();entry.finish(descriptor).unwrap();
    }
    let original = zip.finish().unwrap();
    let source = root.0.join("original.pptx");
    fs::write(&source, &original).unwrap();
    let binary = env!("CARGO_BIN_EXE_mo-cli");
    let inspection = Command::new(binary)
        .arg("opc-repair-inspect")
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        inspection.status.success(),
        "{}",
        String::from_utf8_lossy(&inspection.stderr)
    );
    let report: Value = serde_json::from_slice(&inspection.stdout).unwrap();
    assert_eq!(report["applied"], false);
    assert_eq!(report["removedOverrides"].as_array().unwrap().len(), 1);
    let expected = report["sourceSha256"].as_str().unwrap();
    let output = root.0.join("repaired.pptx");
    let apply = |digest: &str, dest: &PathBuf| {
        Command::new(binary)
            .arg("opc-repair-apply")
            .arg(&source)
            .arg(digest)
            .arg(dest)
            .output()
            .unwrap()
    };
    assert!(!apply(&"0".repeat(64), &output).status.success());
    assert!(!output.exists());
    assert!(!apply(expected, &source).status.success());
    assert_eq!(fs::read(&source).unwrap(), original);
    let completed = apply(expected, &output);
    assert!(
        completed.status.success(),
        "{}",
        String::from_utf8_lossy(&completed.stderr)
    );
    let report: Value = serde_json::from_slice(&completed.stdout).unwrap();
    assert_eq!(report["applied"], true);
    let bytes = fs::read(&output).unwrap();
    let package = mo_opc::Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        report["outputSha256"],
        serde_json::to_value(package.sha256()).unwrap()
    );
    assert!(!apply(expected, &output).status.success());
    assert_eq!(fs::read(&output).unwrap(), bytes);
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        fs::read_dir(&root.0).unwrap().count(),
        2,
        "failed writes leave no spools"
    );
}
