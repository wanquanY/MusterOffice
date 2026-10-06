#!/usr/bin/env python3
"""Build a local immutable native media-worker bundle from verified components.

This is a development bundle, not a signed public release or LGPL distribution
package. Binary redistribution also requires exact upstream sources and notices.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(component):
    record = json.loads((component / "native-build.json").read_text())
    upstream = json.loads((component / "ffmpeg-build.json").read_text())
    lock = json.loads((ROOT / "components/ffmpeg/lock.json").read_text())
    target = {"os": {"Darwin": "macos", "Linux": "linux"}[platform.system()],
              "arch": {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64"}[platform.machine()]}
    assert record["upstream"] == upstream and record["lock"] == upstream["lock"] == lock
    assert record["sanitizers"] is False and upstream["sanitizers"] is False
    assert record["nativePlatform"] == upstream["nativePlatform"] == target
    patches = [{"path": str(p.relative_to(ROOT / "components/ffmpeg")),
                "byteLength": p.stat().st_size, "sha256": sha(p)}
               for p in sorted((ROOT / "components/ffmpeg/patches").glob("*.patch"))]
    assert upstream.get("patches") == patches, "stale upstream patch profile"
    sources = record["sources"]
    assert {r["path"] for r in sources} == {"mo_media.c", "mo_media_decode.c", "mo_media.h", "mo_media_internal.h", "mo_encode.c", "mo_encode_mux.c", "mo_encode.h", "mo_encode_internal.h"}
    for base, rows in [(ROOT / "components/ffmpeg", sources), (component, record["artifacts"] + upstream["artifacts"] + upstream["headers"])]:
        for row in rows:
            path = base / row["path"]
            assert path.stat().st_size == row["byteLength"] and sha(path) == row["sha256"], f"stale component {path}"
    return record, target


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--component", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="new directory; never overwrite a prior bundle")
    parser.add_argument("--release", action="store_true")
    args = parser.parse_args()
    component, output = args.component.resolve(), args.output.resolve()
    record, target = verify(component)
    output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ)
    env["MO_FFMPEG_LIB_DIR"] = str(component)
    env.pop("CARGO_TARGET_DIR", None)
    command = ["cargo", "build", "--offline", "--locked", "-p", "mo-media-worker"]
    if args.release:
        command.append("--release")
    result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, timeout=600)
    (output / "build.stdout").write_text(result.stdout)
    (output / "build.stderr").write_text(result.stderr)
    result.check_returncode()
    (output / "bin").mkdir()
    (output / "lib").mkdir()
    worker = output / "bin/mo-media-worker"
    shutil.copy2(ROOT / "target" / ("release" if args.release else "debug") / "mo-media-worker", worker)
    files = [worker]
    for row in record["artifacts"] + record["upstream"]["artifacts"]:
        source = component / row["path"]
        destination = output / "lib" / source.name
        shutil.copy2(source, destination)
        files.append(destination)
    lib = component / "install/lib"
    links = []
    for name, major in record["lock"]["libraries"].items():
        names = [f"lib{name}.dylib", f"lib{name}.{major}.dylib"] if target["os"] == "macos" else [f"lib{name}.so", f"lib{name}.so.{major}"]
        for name in names:
            source = (lib / name).resolve()
            assert any(p.name == source.name and sha(p) == sha(source) for p in files), "unverified loader alias"
            (output / "lib" / name).symlink_to(source.name)
            links.append({"path": "lib/" + name, "target": source.name})
    # The executable is linked only against its sibling bundle, never an absolute
    # development path. Record loader output so this boundary is reviewable.
    if target["os"] == "macos":
        load = subprocess.check_output(["otool", "-l", str(worker)], text=True)
        rpaths = [line.strip().split()[1] for block in load.split("Load command ")[1:] if "cmd LC_RPATH\n" in block
                  for line in block.splitlines() if line.strip().startswith("path ")]
        assert rpaths == ["@executable_path/../lib"], rpaths
        dependencies = subprocess.check_output(["otool", "-L", str(worker)], text=True)
        assert "@rpath/libmo_media.dylib" in dependencies
    else:
        load = subprocess.check_output(["readelf", "-d", str(worker)], text=True)
        rpaths = [line for line in load.splitlines() if "(RPATH)" in line or "(RUNPATH)" in line]
        assert len(rpaths) == 1 and "[$ORIGIN/../lib]" in rpaths[0], rpaths
        assert "[libmo_media.so]" in load
    (output / "loader.txt").write_text(load)
    manifest = {"format": "musteroffice.media-worker-bundle/1", "profile": "musteroffice.native-media-worker/3", **target,
                "componentRecordSha256": sha(component / "native-build.json"),
                "files": [{"path": str(p.relative_to(output)), "byteLength": p.stat().st_size, "sha256": sha(p)} for p in files], "links": links}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    for file in files:
        file.chmod(0o555 if file == worker else 0o444)
    for name in ["bin", "lib"]:
        (output / name).chmod(0o555)
    report = {"status": "built", "manifestSha256": sha(output / "manifest.json"), "command": command,
              "profile": "release" if args.release else "debug", "artifactBytes": sum(r["byteLength"] for r in manifest["files"])}
    (output / "bundle-build.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
