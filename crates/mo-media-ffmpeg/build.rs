use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn read(path: &Path) -> Vec<u8> {
    println!("cargo:rerun-if-changed={}", path.display());
    fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
fn json(path: &Path) -> Value {
    serde_json::from_slice(&read(path)).expect("component build JSON")
}
fn verify(base: &Path, record: &Value) -> PathBuf {
    let relative = Path::new(record["path"].as_str().expect("artifact path"));
    assert!(
        relative
            .components()
            .all(|p| matches!(p, std::path::Component::Normal(_))),
        "invalid component path"
    );
    let path = base.join(relative);
    let bytes = read(&path);
    assert_eq!(
        record["byteLength"].as_u64(),
        Some(bytes.len() as u64),
        "{} length",
        path.display()
    );
    assert_eq!(
        record["sha256"],
        format!("{:x}", Sha256::digest(&bytes)),
        "{} digest; rebuild stale component",
        path.display()
    );
    path.canonicalize().expect("artifact canonical path")
}
fn main() {
    println!("cargo:rerun-if-env-changed=MO_FFMPEG_LIB_DIR");
    println!("cargo:rerun-if-env-changed=CC");
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().unwrap();
    let directory = env::var_os("MO_FFMPEG_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".codex-work/ffmpeg-native"))
        .canonicalize()
        .expect("build pinned media component first; see components/ffmpeg/README.md");
    let record = json(&directory.join("native-build.json"));
    let upstream = json(&directory.join("ffmpeg-build.json"));
    let lock = json(&root.join("components/ffmpeg/lock.json"));
    assert_eq!(record["format"], "musteroffice.ffmpeg-bridge/1");
    assert_eq!(upstream["format"], "musteroffice.ffmpeg-libraries/1");
    assert_eq!(
        record["upstream"], upstream,
        "embedded upstream build identity"
    );
    assert_eq!(
        env::var("HOST").unwrap(),
        env::var("TARGET").unwrap(),
        "cross builds require a verified target ABI profile"
    );
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    assert!(
        matches!(os.as_str(), "linux" | "macos"),
        "target component profile unavailable"
    );
    for profile in [&record, &upstream] {
        assert_eq!(profile["lock"], lock, "source lock mismatch");
        assert_eq!(
            profile["sanitizers"], false,
            "sanitized libraries require a separate linker profile"
        );
        assert_eq!(profile["nativePlatform"]["os"], os, "OS mismatch");
        assert_eq!(
            profile["nativePlatform"]["arch"],
            env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
            "architecture mismatch"
        );
    }
    let sources = record["sources"].as_array().expect("wrapper sources");
    let names: BTreeSet<_> = sources
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        BTreeSet::from([
            "mo_media.c",
            "mo_media_decode.c",
            "mo_media.h",
            "mo_media_internal.h"
        ])
    );
    assert_eq!(names.len(), sources.len(), "duplicate wrapper source");
    for row in sources {
        verify(&root.join("components/ffmpeg"), row);
    }
    let libraries: BTreeSet<_> = record["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .chain(upstream["artifacts"].as_array().unwrap())
        .map(|row| verify(&directory, row))
        .collect();
    assert_eq!(libraries.len(), 6, "five libraries and one bridge required");
    let headers = upstream["headers"].as_array().expect("upstream headers");
    assert!(!headers.is_empty(), "missing header inventory");
    for row in headers {
        verify(&directory, row);
    }
    let lib = directory.join("install/lib");
    // Check the actual loader aliases as well as the versioned artifacts. A stale
    // unversioned/major symlink must not select an unverified library at runtime.
    println!("cargo:rerun-if-changed={}", lib.display());
    for (name, major) in lock["libraries"].as_object().unwrap() {
        for alias in if os == "macos" {
            vec![
                format!("lib{name}.dylib"),
                format!("lib{name}.{major}.dylib"),
            ]
        } else {
            vec![format!("lib{name}.so"), format!("lib{name}.so.{major}")]
        } {
            assert!(
                libraries.contains(&lib.join(alias).canonicalize().expect("loader alias")),
                "unverified library alias"
            );
        }
    }
    let bridge = lib.join(if os == "macos" {
        "libmo_media.dylib"
    } else {
        "libmo_media.so"
    });
    assert!(libraries.contains(&bridge.canonicalize().unwrap()));
    // A C compiler measures the pinned public ABI. Generated const assertions
    // validate every Rust field offset, struct size and alignment, not just totals.
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let probe = out.join("media-abi-probe");
    println!("cargo:rerun-if-changed=abi_probe.c");
    let result = Command::new(env::var_os("CC").unwrap_or_else(|| "clang".into()))
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(root.join("components/ffmpeg"))
        .arg(manifest.join("abi_probe.c"))
        .arg("-o")
        .arg(&probe)
        .output()
        .expect("C ABI compiler");
    assert!(
        result.status.success(),
        "ABI probe compilation: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new(probe).output().expect("C ABI probe");
    assert!(result.status.success(), "ABI probe execution");
    fs::write(out.join("abi_layout.rs"), result.stdout).expect("write ABI assertions");
    println!(
        "cargo:rustc-env=MO_MEDIA_EXPECTED_VERSION={}",
        lock["version"].as_str().unwrap()
    );
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=dylib=mo_media");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib.display());
    // Consumers with executables add their own development/distribution rpath;
    // link-arg is intentionally not presented as transitive Cargo metadata.
    println!("cargo::metadata=library_dir={}", lib.display());
}
