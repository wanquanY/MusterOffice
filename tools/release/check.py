"""Export with the actual registry SDK and paired worker, then inspect stored bytes.

The operator supplies owned input fixtures (request.json, snapshot.json and
assets.json). This does not start a product, access a database or publish a release.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def checked_file(root, record):
    path = root / record["path"]
    if (not path.resolve().is_relative_to(root) or path.is_symlink()
            or not path.is_file() or path.stat().st_size != record["byteLength"]
            or digest(path) != record["sha256"]):
        raise ValueError("release file differs from its pinned record")
    return path


def external_packages(lock):
    return {(p["name"], p["version"], p.get("source"), p.get("checksum"))
            for p in lock["package"] if "source" in p and not p["name"].startswith("mo-")}


def check(release, expected, target, inputs, output):
    release, inputs, output = release.resolve(), inputs.resolve(), output.resolve()
    manifest_path = release / "release.json"
    if manifest_path.is_symlink() or digest(manifest_path) != expected:
        raise ValueError("release manifest differs from the explicit pin")
    manifest = json.loads(manifest_path.read_bytes())
    if manifest["format"] != "musteroffice.release/1":
        raise ValueError("unsupported release")
    worker_record = manifest["workers"][target]
    worker = checked_file(release, worker_record)
    sdk = manifest["sdk"]
    if sdk["rootCrate"] != "mo-embedded-sdk" or sdk["version"] != manifest["version"]:
        raise ValueError("SDK release identity differs")
    pinned = {}
    for package in sdk["packages"]:
        checked_file(release, package)
        if package["version"] != sdk["version"] or package["name"] in pinned:
            raise ValueError("duplicate or mismatched registry package")
        pinned[package["name"]] = package["sha256"]
    if any(output.is_relative_to(root) for root in [release, inputs, ROOT / "crates", ROOT / "tools"]):
        raise ValueError("output overlaps verification inputs")
    output.mkdir(parents=True, exist_ok=False)
    # Distribution inventories bind bytes, not the storage medium's executable
    # bit. Materialize a private executable without mutating the immutable release.
    executable = output / worker.name
    shutil.copyfile(worker, executable)
    executable.chmod(0o700)
    if digest(executable) != worker_record["sha256"]:
        raise ValueError("worker changed during preparation")
    consumer = output / "consumer"
    (consumer / "src").mkdir(parents=True)
    (consumer / ".cargo").mkdir()
    # Exercise the public example without compiling any source/path dependency.
    shutil.copyfile(ROOT / "tools/sdk/example/src/main.rs", consumer / "src/main.rs")
    q = json.dumps
    (consumer / "Cargo.toml").write_text(
        '[package]\nname="musteroffice-release-check"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[dependencies]\n'
        f'mo-embedded-sdk={{version={q("=" + sdk["version"])},registry="release"}}\n'
        'serde_json="=1.0.151"\n')
    (consumer / ".cargo/config.toml").write_text(
        f'[registries.release]\nindex={q(sdk["registry"])}\n'
        f'[source.release]\nregistry={q(sdk["registry"])}\nreplace-with="local-release"\n'
        f'[source.local-release]\nlocal-registry={q(str(release / "registry"))}\n')
    # Retain the producer's external resolution. Offline metadata may only remove
    # unused packages or resolve owned registry versions; it cannot upgrade it.
    original_lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    shutil.copyfile(ROOT / "Cargo.lock", consumer / "Cargo.lock")
    with (output / "cargo.log").open("wb") as log:
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--offline", "--format-version=1"], cwd=consumer, stderr=log))
        lock = tomllib.loads((consumer / "Cargo.lock").read_text())
        if not external_packages(lock) <= external_packages(original_lock):
            raise ValueError("external Cargo resolution changed")
        owned = [p for p in metadata["packages"] if p["name"].startswith("mo-")]
        if not owned or any(p["version"] != sdk["version"] or p["source"] is None for p in owned):
            raise ValueError("consumer did not resolve the actual release packages")
        resolved = {p["name"]: p.get("checksum") for p in lock["package"] if p["name"].startswith("mo-")}
        if resolved != pinned:
            raise ValueError("resolved registry checksums differ from release pins")
        subprocess.run(["cargo", "run", "--locked", "--offline", "--quiet", "--",
                        str(inputs), str(output / "delivery"), str(executable), worker_record["sha256"]],
                       cwd=consumer, stdout=log, stderr=log, check=True)
    report = dict(format="musteroffice.release-export-check/1", releaseManifestSha256=expected,
                  version=sdk["version"], target=target, workerSha256=worker_record["sha256"],
                  registryPackages=len(owned), inspectionSha256=digest(output / "delivery/inspection.json"),
                  status="passed")
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(check(args.release, args.sha256, args.target, args.input, args.output)))
