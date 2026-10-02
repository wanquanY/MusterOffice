"""Assemble an immutable integration release from independently verified inputs.

This command neither builds the receiving product nor publishes anything. An
integration release binds bytes; it does not claim Office/WPS or product acceptance.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil

from cargo import build_registry

ROOT = Path(__file__).resolve().parents[2]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def file_record(path, root):
    return dict(path=path.relative_to(root).as_posix(), sha256=sha(path), byteLength=path.stat().st_size)


def copy(source, target, expected, *, executable=False):
    if source.is_symlink() or not source.is_file() or sha(source) != expected:
        raise ValueError("input differs from its explicit SHA-256")
    target.parent.mkdir(parents=True, exist_ok=True)
    with source.open("rb") as incoming, target.open("xb") as outgoing:
        shutil.copyfileobj(incoming, outgoing)
    if sha(target) != expected:
        raise ValueError("input changed during assembly")
    # Byte digests do not carry filesystem mode. A release must be runnable
    # directly as well as after installation by a receiving product.
    target.chmod(0o755 if executable else 0o644)


def build(*, sdk, sdk_sha256, playback, playback_sha256, workers, output, version, registry):
    sdk = sdk.resolve()
    playback = playback.resolve()
    output = output.resolve()
    if any(output.is_relative_to(p) for p in [sdk, playback, ROOT / "crates", ROOT / "tools"]):
        raise ValueError("output overlaps source inputs")
    sdk_manifest = module("sdk_verify", ROOT / "tools/sdk/verify.py").verify(sdk, sdk_sha256)
    playback_manifest = module("playback_build", ROOT / "tools/playback-sdk/build.py").verify(playback, playback_sha256)
    output.mkdir(parents=True, exist_ok=False)
    packages = build_registry(sdk, sdk_manifest, output / "registry", version, registry)
    native = {}
    supported = {f"{platform}-{arch}" for platform in ["darwin", "linux", "win32"] for arch in ["arm64", "x64"]}
    for target, source, expected in workers:
        if target not in supported or target in native or not re.fullmatch("[a-f0-9]{64}", expected):
            raise ValueError("invalid or duplicate native target")
        name = "mo-export-worker" + (".exe" if target.startswith("win32-") else "")
        destination = output / "workers" / target / name
        copy(source, destination, expected, executable=not target.startswith("win32-"))
        native[target] = file_record(destination, output)
    if not native:
        raise ValueError("at least one explicitly pinned worker required")
    for entry in playback_manifest["files"]:
        copy(playback / entry["path"], output / "playback" / entry["path"], entry["sha256"])
    copy(playback / "bundle-manifest.json", output / "playback/bundle-manifest.json", playback_sha256)
    manifest = dict(
        format="musteroffice.release/1", version=version, channelEligibility="development",
        source=dict(sdkManifestSha256=sdk_sha256),
        compatibility=dict(nativeProtocol="musteroffice.native-export/2-draft",
                           computationProtocol="musteroffice.computation/1-draft"),
        sdk=dict(registry=registry, rootCrate="mo-embedded-sdk", version=version, packages=packages),
        workers=native,
        playback=dict(manifestSha256=playback_sha256,
                      files=[file_record(p, output) for p in sorted((output / "playback").rglob("*")) if p.is_file()]),
    )
    raw = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
    (output / "release.json").write_bytes(raw)
    return dict(directory=str(output), manifestSha256=hashlib.sha256(raw).hexdigest(), version=version,
                packages=len(packages), targets=sorted(native))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk", type=Path, required=True)
    parser.add_argument("--sdk-sha256", required=True)
    parser.add_argument("--playback", type=Path, required=True)
    parser.add_argument("--playback-sha256", required=True)
    parser.add_argument("--worker", action="append", nargs=3, metavar=("TARGET", "PATH", "SHA256"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--registry", required=True, help="Consumer-owned canonical sparse HTTPS registry")
    args = parser.parse_args()
    print(json.dumps(build(sdk=args.sdk, sdk_sha256=args.sdk_sha256, playback=args.playback,
                          playback_sha256=args.playback_sha256,
                          workers=[(target, Path(file), digest) for target, file, digest in args.worker],
                          output=args.output, version=args.version, registry=args.registry)))


if __name__ == "__main__":
    main()
