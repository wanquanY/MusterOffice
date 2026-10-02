"""Seal a Linux native component for OCI distribution after execution acceptance.

The integration release also contains experimental browser assets. Qualification
here is deliberately scoped to the native SDK/worker pair; it never promotes the
browser SDK or claims Office/WPS interoperability or receiving-product acceptance.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil

from check import checked_file, digest

PROFILE = "linux-native-presentation/1"
TARGET = "linux-x64"


def record(path, root):
    return {"path": path.relative_to(root).as_posix(), "sha256": digest(path),
            "byteLength": path.stat().st_size}


def read_json(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 4 * 1024 * 1024:
        raise ValueError("invalid bounded evidence file")
    return json.loads(path.read_bytes())


def verify_build(directory, worker):
    report = read_json(directory / "build.json")
    source = read_json(directory / "source.json")
    if (report.get("format") != "musteroffice.linux-worker-build/1"
            or report.get("target") != TARGET
            or report.get("worker") != {key: worker[key] for key in ["sha256", "byteLength"]}
            or source.get("buildReportSha256") != digest(directory / "build.json")):
        raise ValueError("Linux build evidence does not bind this worker")
    files = ["build.json", "source.json"]
    for name, expected in [("mo-export-worker", report["worker"]),
                           ("tests.log", report["tests"]),
                           ("dynamic-libraries.txt", report["dynamicLibraries"]),
                           *[(key + "-build.json", item) for key, item in report["components"].items()]]:
        checked_file(directory, {"path": name, **expected})
        if name != "mo-export-worker":
            files.append(name)
    if set(report["components"]) != {"skia", "harfbuzz", "imageCodecs"}:
        raise ValueError("incomplete native dependency evidence")
    if "not found" in (directory / "dynamic-libraries.txt").read_text():
        raise ValueError("unresolved native libraries")
    return source["sourceRevision"], files


def verify_export(directory, release_sha256, manifest, worker):
    report = read_json(directory / "report.json")
    if (report.get("format") != "musteroffice.release-export-check/1"
            or report.get("status") != "passed"
            or report.get("target") != TARGET
            or report.get("releaseManifestSha256") != release_sha256
            or report.get("version") != manifest["version"]
            or report.get("workerSha256") != worker["sha256"]
            or report.get("registryPackages") != len(manifest["sdk"]["packages"])
            or report.get("inspectionSha256") != digest(directory / "delivery/inspection.json")):
        raise ValueError("registry SDK execution does not bind this release")
    # The delivery checker opens the stored result, not just an in-memory model.
    # Preserve those exact public receipts with the exported component.
    return ["report.json", "cargo.log", "delivery/inspection.json"]


def assemble(release, expected, build, export, output):
    release, build, export, output = [p.resolve() for p in [release, build, export, output]]
    if digest(release / "release.json") != expected:
        raise ValueError("integration release pin differs")
    manifest = read_json(release / "release.json")
    if manifest.get("format") != "musteroffice.release/1":
        raise ValueError("unsupported integration release")
    worker = manifest["workers"][TARGET]
    checked_file(release, worker)
    revision, build_files = verify_build(build, worker)
    export_files = verify_export(export, expected, manifest, worker)
    if any(output.is_relative_to(root) for root in [release, build, export]):
        raise ValueError("output overlaps immutable inputs")
    output.mkdir(parents=True, exist_ok=False)
    component = output / "component"
    component.mkdir()

    def copy(source, relative):
        destination = component / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        destination.chmod(0o644)

    copy(release / "release.json", "release.json")
    for package in manifest["sdk"]["packages"]:
        copy(checked_file(release, package), package["path"])
        name = package["name"]
        index = f"registry/index/{name[:2]}/{name[2:4]}/{name}"
        if read_json(release / index) != package["index"]:
            raise ValueError("registry index differs from release")
        copy(release / index, index)
    copy(release / worker["path"], worker["path"])
    (component / worker["path"]).chmod(0o755)
    for name in build_files:
        copy(build / name, "evidence/build/" + name)
    for name in export_files:
        copy(export / name, "evidence/export/" + name)
    qualified = {
        "format": "musteroffice.native-component/1", "version": manifest["version"],
        "target": TARGET, "sourceRevision": revision,
        "releaseManifestSha256": expected,
        "qualification": {"profile": PROFILE, "status": "verified",
                          "buildEvidence": "evidence/build/build.json",
                          "exportEvidence": "evidence/export/report.json"},
        "scope": ["registry-sdk-native-worker-compatibility", "native-execution-and-lifecycle",
                  "stored-pptx-and-preview-inspection"],
        "notProven": ["browser-playback", "Office-WPS-interoperability", "receiving-product-acceptance"],
        "files": [record(p, component) for p in sorted(component.rglob("*")) if p.is_file()],
    }
    raw = (json.dumps(qualified, sort_keys=True, indent=2) + "\n").encode()
    (component / "native-component.json").write_bytes(raw)
    shutil.copyfile(Path(__file__).with_name("native-image.Dockerfile"), output / "Dockerfile")
    return {"version": qualified["version"], "sourceRevision": revision,
            "componentSha256": hashlib.sha256(raw).hexdigest(), "directory": str(output)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--export-check", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(assemble(args.release, args.sha256, args.build, args.export_check, args.output)))
