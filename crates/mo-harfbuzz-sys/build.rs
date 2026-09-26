use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=MO_HARFBUZZ_LIB_DIR");
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let directory = env::var_os("MO_HARFBUZZ_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".codex-work/harfbuzz/release"));
    let directory=directory.canonicalize().expect("build pinned native HarfBuzz component first; see docs/implementation/harfbuzz-component.md");
    let record = directory.join("native-build.json");
    let library = directory.join("libmo_harfbuzz.a");
    println!("cargo:rerun-if-changed={}", record.display());
    println!("cargo:rerun-if-changed={}", library.display());
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(record).expect("component build record"))
            .expect("component build JSON");
    let lock: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("components/harfbuzz/lock.json")).unwrap())
            .unwrap();
    println!(
        "cargo:rerun-if-changed={}",
        root.join("components/harfbuzz/lock.json").display()
    );
    assert_eq!(record["lock"], lock, "component source lock mismatch");
    assert_eq!(
        record["faultTests"], false,
        "native worker must not link fault injection"
    );
    assert_eq!(
        record["sanitizers"], false,
        "sanitized component needs its own linker profile"
    );
    let bytes = fs::read(&library).expect("component archive");
    let artifact = record["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| {
            a["path"]
                .as_str()
                .is_some_and(|p| p.ends_with("/libmo_harfbuzz.a"))
        })
        .expect("archive digest record");
    assert_eq!(
        artifact["sha256"],
        format!("{:x}", Sha256::digest(&bytes)),
        "archive digest mismatch"
    );
    assert_eq!(artifact["byteLength"].as_u64(), Some(bytes.len() as u64));
    assert_eq!(
        env::var("HOST").unwrap(),
        env::var("TARGET").unwrap(),
        "cross builds require a separate verified component target profile"
    );
    println!("cargo:rustc-link-search=native={}", directory.display());
    println!("cargo:rustc-link-lib=static=mo_harfbuzz");
    match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "macos" => println!("cargo:rustc-link-lib=c++"),
        "linux" => println!("cargo:rustc-link-lib=stdc++"),
        _ => panic!("native component target runtime profile not yet implemented"),
    }
}
