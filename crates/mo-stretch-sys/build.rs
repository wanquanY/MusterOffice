use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::Path};

fn read(path: &Path) -> Vec<u8> {
    println!("cargo:rerun-if-changed={}", path.display());
    fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
fn json(path: &Path) -> Value {
    serde_json::from_slice(&read(path)).expect("DSP build JSON")
}
fn verify(base: &Path, row: &Value) {
    let path = Path::new(row["path"].as_str().expect("artifact path"));
    assert!(
        path.components()
            .all(|p| matches!(p, std::path::Component::Normal(_)))
    );
    let bytes = read(&base.join(path));
    assert_eq!(row["byteLength"].as_u64(), Some(bytes.len() as u64));
    assert_eq!(
        row["sha256"],
        format!("{:x}", Sha256::digest(&bytes)),
        "stale DSP artifact"
    );
}
fn main() {
    println!("cargo:rerun-if-env-changed=MO_STRETCH_LIB_DIR");
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../..")
        .canonicalize()
        .unwrap();
    let dir = env::var_os("MO_STRETCH_LIB_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join(".codex-work/stretch-native"));
    let record = json(&dir.join("build.json"));
    assert_eq!(record["format"], "musteroffice.stretch-build/1");
    assert_eq!(
        record["builder"]["path"],
        "tools/components/build-stretch.py"
    );
    verify(&root, &record["builder"]);
    assert_eq!(
        record["lock"],
        json(&root.join("components/signalsmith-stretch/lock.json"))
    );
    assert_eq!(record["target"], "native");
    assert_eq!(record["sanitizers"], false);
    assert_eq!(record["faultTests"], false);
    assert_eq!(
        env::var("HOST").unwrap(),
        env::var("TARGET").unwrap(),
        "unverified DSP cross build"
    );
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    assert!(matches!(os.as_str(), "macos" | "linux"));
    assert_eq!(record["nativePlatform"]["os"], os);
    assert_eq!(
        record["nativePlatform"]["arch"],
        env::var("CARGO_CFG_TARGET_ARCH").unwrap()
    );
    let sources = record["sources"].as_array().unwrap();
    let names: BTreeSet<_> = sources
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        BTreeSet::from(["mo_stretch.cpp", "mo_stretch.h", "mo_stretch_memory.h"])
    );
    assert_eq!(sources.len(), names.len());
    for row in sources {
        verify(&root.join("components/signalsmith-stretch"), row);
    }
    for row in record["preparedSources"].as_array().unwrap() {
        verify(&dir, row);
    }
    let artifacts = record["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0]["path"], "libmo_stretch.a");
    verify(&dir, &artifacts[0]);
    println!(
        "cargo:rustc-link-search=native={}",
        dir.canonicalize().unwrap().display()
    );
    println!("cargo:rustc-link-lib=static=mo_stretch");
    println!(
        "cargo:rustc-link-lib=dylib={}",
        if os == "macos" { "c++" } else { "stdc++" }
    );
}
