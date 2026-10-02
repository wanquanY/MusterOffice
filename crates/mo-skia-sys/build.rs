use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn verify(root: &Path, entry: &serde_json::Value) -> PathBuf {
    let p = root.join(entry["path"].as_str().expect("artifact path"));
    println!("cargo:rerun-if-changed={}", p.display());
    let bytes = fs::read(&p).expect("verified component file");
    assert_eq!(
        entry["sha256"],
        format!("{:x}", Sha256::digest(&bytes)),
        "component hash: {}",
        p.display()
    );
    assert_eq!(entry["byteLength"].as_u64(), Some(bytes.len() as u64));
    p
}
fn main() {
    println!("cargo:rerun-if-env-changed=MO_SKIA_LIB_DIR");
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../..")
        .canonicalize()
        .unwrap();
    let directory = env::var_os("MO_SKIA_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".codex-work/skia"));
    let record_path = directory.join("native-build.json");
    println!("cargo:rerun-if-changed={}", record_path.display());
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(record_path).expect("build pinned native Skia first"))
            .unwrap();
    let lock_path = root.join("components/skia/lock.json");
    println!("cargo:rerun-if-changed={}", lock_path.display());
    let lock: serde_json::Value = serde_json::from_slice(&fs::read(lock_path).unwrap()).unwrap();
    assert_eq!(record["lock"], lock, "Skia lock mismatch");
    assert_eq!(record["target"], "native");
    assert_eq!(record["sanitizers"], false);
    assert_eq!(
        env::var("HOST").unwrap(),
        env::var("TARGET").unwrap(),
        "separate cross target profile required"
    );
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    assert!(
        matches!(target_os.as_str(), "macos" | "linux"),
        "unsupported native raster platform"
    );
    if target_os == "linux" || record.get("nativePlatform").is_some() {
        assert_eq!(
            record["nativePlatform"]["os"], target_os,
            "component OS mismatch"
        );
        assert_eq!(
            record["nativePlatform"]["arch"], target_arch,
            "component architecture mismatch"
        );
        assert_eq!(
            record["imageCodecs"]["nativePlatform"], record["nativePlatform"],
            "codec platform mismatch"
        );
    }
    for source in record["componentSources"].as_array().unwrap() {
        verify(&root, source);
    }
    let codec_lock: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("components/image-codec/lock.json")).unwrap())
            .unwrap();
    assert_eq!(
        record["imageCodecs"]["lock"], codec_lock,
        "image codec lock mismatch"
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("components/image-codec/lock.json").display()
    );
    for (suffix, name) in [
        ("/libmo_skia_adapter.a", "mo_skia_adapter"),
        ("/libskia.a", "skia"),
        ("/libpng16.a", "png16"),
        ("/libjpeg.a", "jpeg"),
        ("/libz.a", "z"),
    ] {
        let artifact = record["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["path"].as_str().is_some_and(|p| p.ends_with(suffix)))
            .expect("component archive record");
        let library = verify(&root, artifact);
        println!(
            "cargo:rustc-link-search=native={}",
            library.parent().unwrap().display()
        );
        println!("cargo:rustc-link-lib=static={name}");
    }
    println!(
        "cargo:rustc-link-lib={}",
        if target_os == "macos" {
            "c++"
        } else {
            "stdc++"
        }
    );
}
