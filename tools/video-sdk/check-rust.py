"""Execute the real video SDK through its Cargo registry, without source paths."""
import argparse
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

spec = importlib.util.spec_from_file_location("video_rust_builder", Path(__file__).with_name("rust.py"))
sdk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sdk)

PROGRAM = '''use mo_video_operations::{*, edit::{EditContract, Operation}};
fn main() {
    let check = || false;
    let input = r#"{"documentId":"native-consumer","title":"原生工程","rootSequence":"main","canvas":{"width":1280,"height":720},"frameRate":{"numerator":30000,"denominator":1001},"audioSampleRate":48000}"#;
    let project = VideoProject::create(decode(input, 4096, &check).unwrap(), Limits::default(), &check).unwrap();
    let candidate = project.prepare(&Transaction { contract: EditContract::V1, expected_digest: project.digest().clone(), operations: vec![Operation::SetTitle {title:"已编辑".into()}]}, &check).unwrap();
    assert_eq!(project.summary(&check).unwrap().title,"原生工程");
    let changed = VideoProject::open(candidate.snapshot.clone(),Limits::default(),&check).unwrap();
    let undo = changed.prepare_inverse(&InverseRequest { expected_digest:changed.digest().clone(),inverse:candidate.inverse},&check).unwrap();
    assert_eq!(undo.snapshot,project.snapshot());
    assert_eq!(changed.summary(&check).unwrap().title,"已编辑");
    assert_eq!(changed.resources(&check).unwrap().resources.len(),0);
    let page = changed.list(PageRequest { query:ListQuery::Sequences,cursor:None,limit:10 },&check).unwrap();
    assert_eq!(page.items.len(),1);
    let outside = changed.frame(FrameRequest { expected_digest:changed.digest().clone(),frame:Ticks::new(0)},&check).unwrap_err();
    assert_eq!(outside.code,FailureCode::OutsideProject);
    let mut corrupt = changed.snapshot();corrupt.document.title="tampered".into();
    assert_eq!(VideoProject::open(corrupt,Limits::default(),&check).err().unwrap().code,FailureCode::SnapshotCorrupt);
    println!("{}", encode(&changed.summary(&check).unwrap(),4096,&check).unwrap());
}
'''


def check(bundle, pin, output):
    manifest = sdk.verify(bundle, pin)
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    # Only registry archives/index reach the consumer. The source SDK directory
    # cannot accidentally satisfy a path dependency in this execution.
    shutil.copytree(bundle / "registry", output / "registry")
    consumer = output / "consumer"
    (consumer / "src").mkdir(parents=True)
    (consumer / ".cargo").mkdir()
    registry = manifest["registry"]
    config = {"registries": {"video": {"index": registry}},
              "source": {"video": {"registry": registry, "replace-with": "pinned"},
                         "pinned": {"local-registry": str(output / "registry")}}}
    (consumer / ".cargo/config.toml").write_bytes(sdk.cargo.toml_bytes(config))
    cargo = {"workspace": {}, "package": {"name": "video-sdk-consumer", "version": "0.0.0", "edition": "2024"},
             "dependencies": {"mo-video-operations": {"version": "=" + manifest["version"], "registry": "video"}}}
    (consumer / "Cargo.toml").write_bytes(sdk.cargo.toml_bytes(cargo))
    (consumer / "src/main.rs").write_text(PROGRAM)
    shutil.copyfile(bundle / "source/Cargo.lock", consumer / "Cargo.lock")
    completed = subprocess.run(["cargo", "run", "--offline", "--quiet"], cwd=consumer, capture_output=True, text=True)
    (output / "cargo.log").write_text(completed.stderr)
    completed.check_returncode()
    summary = json.loads(completed.stdout)
    lock = tomllib.loads((consumer / "Cargo.lock").read_text())
    packages = {p["name"]: p for p in lock["package"] if p["name"].startswith("mo-")}
    if set(packages) != {p["name"] for p in manifest["packages"]}:
        raise ValueError("consumer Cargo closure differs")
    for item in manifest["packages"]:
        package = packages[item["name"]]
        if package.get("source") != registry or package.get("version") != item["version"] or package.get("checksum") != item["sha256"]:
            raise ValueError("consumer Cargo package is not pinned")
    before = tomllib.loads((bundle / "source/Cargo.lock").read_text())
    external = lambda rows: {(p["name"], p["version"], p.get("source"), p.get("checksum")) for p in rows["package"] if p.get("source") and not p["name"].startswith("mo-")}
    if external(lock) != external(before):
        raise ValueError("consumer external Cargo graph drifted")
    report = {"format": "musteroffice.video-rust-consumer/1", "status": "passed", "manifestSha256": pin,
              "version": manifest["version"], "packages": len(packages), "summary": summary,
              "consumerLockSha256": sdk.wasm_builder.sha(consumer / "Cargo.lock"),
              "scope": "Actual independent registry crates and fixed external Cargo graph; create, edit, undo, summary, resources, query and refusal behavior. No product authority/media/UI acceptance."}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(check(args.bundle, args.sha256, args.output)))
