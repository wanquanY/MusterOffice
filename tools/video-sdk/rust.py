"""Build the independent video computation Cargo closure as a local registry.

No publishing, product code, private paths in manifests, codecs or build scripts.
The receiving product uses an external manifest pin and Cargo source replacement.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
ROOT_CRATE = "mo-video-operations"
FORMAT = "musteroffice.video-rust-sdk/1-draft"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


source_builder = module("portable_source_builder", ROOT / "tools/sdk/build.py")
cargo = module("cargo_package_builder", ROOT / "tools/release/cargo.py")
wasm_builder = module("video_wasm_builder", ROOT / "tools/video-sdk/build.py")


def closure():
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]
    declarations = workspace["dependencies"]
    pending, selected, dependencies = [ROOT_CRATE], {}, set()
    while pending:
        name = pending.pop()
        if name in selected:
            continue
        directory = ROOT / declarations[name]["path"]
        if directory.is_symlink() or not directory.resolve().is_relative_to(ROOT / "crates"):
            raise ValueError("video crate path leaves owned source")
        manifest = tomllib.loads((directory / "Cargo.toml").read_text())
        if (manifest["package"]["name"] != name or (directory / "build.rs").exists()
                or any(key in manifest for key in ["build-dependencies", "bin", "example", "test", "bench"])
                or "build" in manifest["package"]):
            raise ValueError("video SDK must contain only portable computation libraries")
        production = source_builder.production_dependencies(manifest)
        manifest["package"].update(workspace="../..", autobins=False, autoexamples=False, autotests=False, autobenches=False)
        manifest.setdefault("lib", {}).update(test=False, doctest=False, bench=False)
        selected[name] = (directory, manifest)
        for key, dependency in production:
            if not isinstance(dependency, dict) or dependency.get("workspace") is not True:
                raise ValueError("video source dependency must be explicitly inherited")
            dependencies.add(key)
            if isinstance(declarations[key], dict) and "path" in declarations[key]:
                pending.append(key)
    expected = {"mo-common", "mo-time", "mo-video-model", "mo-video-edit", "mo-video-compile", ROOT_CRATE}
    if set(selected) != expected:
        raise ValueError("video computation dependency closure changed; review packaging profile")
    generated = {"workspace": {"resolver": workspace["resolver"], "package": workspace["package"],
                  "members": [declarations[name]["path"] for name in sorted(selected)],
                  "dependencies": {key: declarations[key] for key in sorted(dependencies)}, "lints": workspace["lints"]}}
    return selected, generated


def verify(root, pin):
    path = root / "video-rust-manifest.json"
    if root.is_symlink() or path.is_symlink() or not path.is_file() or path.stat().st_size > 4 << 20 or wasm_builder.sha(path) != pin:
        raise ValueError("video Rust manifest differs from external pin")
    manifest = json.loads(path.read_text())
    if manifest["format"] != FORMAT or manifest["releaseCleared"] is not False or manifest["rootCrate"] != ROOT_CRATE:
        raise ValueError("unsupported video Rust SDK")
    if manifest["files"] != wasm_builder.inventory(root, ("video-rust-manifest.json",)):
        raise ValueError("video Rust SDK inventory differs")
    return manifest


def build(output, version, registry):
    output = output.resolve()
    if any(output.is_relative_to(ROOT / name) for name in ["crates", "tools", "packages", "components", "contracts"]):
        raise ValueError("video SDK output overlaps source")
    selected, workspace = closure()
    inputs = {ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "LICENSE", ROOT / "NOTICE",
              Path(__file__).resolve(), ROOT / "tools/release/cargo.py", ROOT / "tools/sdk/build.py", ROOT / "tools/video-sdk/build.py"}
    for directory, _manifest in selected.values():
        inputs.add(directory / "Cargo.toml")
        for name in ["src", "data"]:
            inputs.update(p for p in (directory / name).rglob("*") if p.is_file())
    inputs.update(p for p in (ROOT / "components/persistent-collections").rglob("*") if p.is_file())
    if any(p.is_symlink() or not p.is_file() or not p.resolve().is_relative_to(ROOT) for p in inputs):
        raise ValueError("video SDK source must be regular owned files")
    before = wasm_builder.record(sorted(inputs))
    output.mkdir(parents=True, exist_ok=False)
    source = output / "source"
    source.mkdir()
    for path in sorted(inputs):
        if path.is_symlink():
            raise ValueError("source symlinks are not allowed")
        relative = path.relative_to(ROOT)
        if relative.parts[0] not in {"crates", "components"} and str(relative) not in {"LICENSE", "NOTICE"}:
            continue
        target = source / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if path.name == "Cargo.toml":
            target.write_text(source_builder.toml_document(selected[path.parent.name][1]))
        else:
            shutil.copyfile(path, target)
    (source / "Cargo.toml").write_text(source_builder.toml_document(workspace))
    shutil.copyfile(ROOT / "Cargo.lock", source / "Cargo.lock")
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--offline", "--format-version=1"], cwd=source, text=True))
    upstream = tomllib.loads((ROOT / "Cargo.lock").read_text())
    actual = tomllib.loads((source / "Cargo.lock").read_text())
    external = lambda lock: {(p["name"], p["version"], p.get("source"), p.get("checksum")) for p in lock["package"] if "source" in p}
    if not external(actual) <= external(upstream):
        raise ValueError("video SDK selected an unpinned external dependency")
    if {p["name"] for p in metadata["packages"] if p["source"] is None} != set(selected):
        raise ValueError("video SDK dependency closure differs")
    source_manifest = {"libraries": sorted(selected), "files": wasm_builder.inventory(source)}
    packages = cargo.build_registry(source, source_manifest, output / "registry", version, registry)
    if wasm_builder.record(sorted(inputs)) != before:
        raise ValueError("source changed while building video SDK")
    manifest = {"format": FORMAT, "releaseCleared": False, "rootCrate": ROOT_CRATE, "version": version,
                "registry": registry, "profile": "musteroffice.video.operations/1-draft",
                "sourceCommit": wasm_builder.run(["git", "rev-parse", "HEAD"]).strip(),
                "sourceDirty": bool(wasm_builder.run(["git", "status", "--porcelain"])),
                "sourceInputs": before, "packages": packages,
                "registryPackages": [{"name": p["name"], "version": p["version"], "source": p["source"], "license": p["license"]} for p in metadata["packages"] if p["source"] is not None],
                "files": wasm_builder.inventory(output),
                "scope": "Independent computation Cargo development package. No codecs, device authority, UI, publishing or formal release acceptance."}
    path = output / "video-rust-manifest.json"
    path.write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    pin = wasm_builder.sha(path)
    verify(output, pin)
    return {"manifestSha256": pin, "packages": len(packages), "sourceDirty": manifest["sourceDirty"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--version")
    parser.add_argument("--registry")
    parser.add_argument("--verify", type=Path)
    parser.add_argument("--sha256")
    args = parser.parse_args()
    if args.verify:
        result = verify(args.verify, args.sha256)
        print(json.dumps({"status": "verified", "packages": len(result["packages"])}))
    else:
        if not all((args.output, args.version, args.registry)):
            parser.error("--output, --version and --registry required")
        print(json.dumps(build(args.output, args.version, args.registry)))


if __name__ == "__main__":
    main()
