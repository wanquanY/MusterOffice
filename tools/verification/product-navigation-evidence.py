"""Verify scoped product navigation evidence without promoting replacement gates."""

import argparse
import hashlib
import json
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--product-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    core = Path(__file__).resolve().parents[2]
    product = args.product_root.resolve()
    stage = product / ".codex-work/integration/product-navigation"

    def label(path):
        path = path.resolve()
        for root, prefix in [(product, "Musterwork:"), (core, "MusterOffice:")]:
            if path.is_relative_to(root):
                return prefix + path.relative_to(root).as_posix()
        raise ValueError(f"path outside owned repositories: {path}")

    def item(path):
        return {
            "path": label(path),
            "sha256": sha(path),
            "byteLength": path.stat().st_size,
        }

    def read(path):
        return json.loads(path.read_bytes())

    commands = []
    source_inventory = {}
    names = [
        "adapter-01",
        "consumer-build-01",
        "preparation-01",
        "client-02",
        "viewer-01",
        "types-01",
        "desktop-build-01",
        "native-agent-01",
        "wide-01",
        "inspection-02",
        "verification-02",
    ]
    runtime_names = {
        "adapter-01",
        "native-agent-01",
        "wide-01",
        "inspection-02",
        "verification-02",
    }
    for name in names:
        record_path = stage / (name + ".json")
        value = read(record_path)
        assert (
            value["sourceUnchanged"] and value["sourceBefore"] == value["sourceAfter"]
        ), name
        assert value["exitCode"] == (101 if name == "native-agent-01" else 0), name
        log = stage / (name + ".log")
        assert sha(log) == value["logSha256"]
        root = product / "apps/agent-runtime" if name in runtime_names else product
        for file, digest in value["sourceAfter"].items():
            path = root / file
            assert sha(path) == digest, label(path)
            source_inventory[label(path)] = digest
        commands.append(
            {
                "name": name,
                "report": item(record_path),
                "log": item(log),
                "exitCode": value["exitCode"],
                "elapsedSeconds": value["elapsedSeconds"],
            }
        )
    lock = read(product / "components/musteroffice/lock.json")
    playback = read(product / "components/musteroffice/playback-lock.json")[
        "manifestSha256"
    ]
    worker_pin = lock["workers"]["darwin-arm64"]
    worker = (
        product
        / "components/musteroffice/generated/workers/darwin-arm64"
        / worker_pin["sha256"]
        / "mo-export-worker"
    )
    assert (
        sha(worker) == worker_pin["sha256"]
        and worker.stat().st_size == worker_pin["byteLength"]
    )
    sdk = product / "apps/agent-runtime/vendor/musteroffice" / lock["sdkManifestSha256"]
    bundle = product / "components/musteroffice/generated/playback" / playback
    for root, manifest, pin in [
        (sdk, "sdk-manifest.json", lock["sdkManifestSha256"]),
        (bundle, "bundle-manifest.json", playback),
    ]:
        path = root / manifest
        assert sha(path) == pin
        for file in read(path)["files"]:
            path = root / file["path"]
            assert (
                sha(path) == file["sha256"]
                and path.stat().st_size == file["byteLength"]
            )
    exported = read(stage / "emitted-01.json")
    assert exported["pin"] == playback
    for file in exported["files"]:
        path = stage / "desktop-build-01/musteroffice" / playback / file["path"]
        assert sha(path) == file["sha256"] and path.stat().st_size == file["byteLength"]
    fixture = stage / "fixtures-02"
    generated = read(fixture / "report.json")
    assert generated["status"] == "passed" and generated["nativeFrames"] == 60
    for name, entry in generated["inputs"].items():
        path = Path(name)
        assert (
            sha(path) == entry["sha256"] and path.stat().st_size == entry["byteLength"]
        )
        source_inventory[label(path)] = entry["sha256"]
    actual = read(stage / "actual-worker-02.json")
    assert actual["frames"] == 95 and len(actual["cases"]) == 5
    for case in actual["cases"]:
        folder = fixture / case["name"]
        assert sha(folder / "contents.bin") == case["packetSha256"]
        assert sha(folder / "request.json") == case["requestSha256"]
        samples = read(folder / "samples.json")
        for sample in samples:
            assert sha(folder / sample["pixels"]) == sample["sha256"]
        hashes = [s["sha256"] for s in samples]
        assert case["frames"] == hashes + [hashes[-1]] + hashes[:6]
    value = {
        "format": "musteroffice.product-navigation-verification/1",
        "date": "2026-09-28",
        "status": "scoped-product-navigation-verified; complete-replacement-open",
        "productBranch": "codex/musteroffice-integration",
        "defaultEngineReplaced": False,
        "sdkManifestSha256": lock["sdkManifestSha256"],
        "playbackManifestSha256": playback,
        "exportWorker": worker_pin,
        "commands": commands,
        "sources": source_inventory,
        "fixtureReport": item(fixture / "report.json"),
        "actualWorkerReport": item(stage / "actual-worker-02.json"),
        "rendererBuild": {
            "files": exported["count"],
            "uncompressedSdkBytes": exported["bytes"],
            "report": item(stage / "emitted-01.json"),
        },
        "passedDistinctTests": 187,
        "productWorkerFrames": 103,
        "newNativeReferenceFrames": 60,
        "testAccounting": "18 preparation + 117 client (5 real dynamic Worker cases) + 7 Vue + 19 adapter + 26 native Agent. Three initial stale-environment fixture failures reran successfully; the extra fitting stress generation repeats one test with different input.",
        "framesAccounting": "95 dynamic Worker samples (60 native references, 5 post-rejection samples, 30 replay samples) plus 8 existing published-version samples. No new browser/Electron observation.",
        "repairedVerification": "Initial 26-case run used older immutable stress outputs with a different renderer digest: 23 passed, 3 rejected. Regenerated both overflow and fitting outputs with the new pinned worker; reinspection and both original final-verification cases passed without changing production guards or tests.",
        "remaining": [
            "full animation and transition families including visibility/set and implicit native events",
            "media, SmartArt, equations and complete advanced content",
            "independent Office/WPS editing and playback acceptance",
            "authenticated Electron session, history migration and complete replacement gates",
            "performance, resource budgets and complete installer measurements",
        ],
        "scope": "Only independent product worktree. Native exports and real SDK/Worker tests; no deployment, publication, default engine switch or complete quality claim.",
    }
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(value, output, indent=2, ensure_ascii=False)
        output.write("\n")
    print(
        json.dumps(
            {
                "evidence": item(args.output),
                "sourceFiles": len(source_inventory),
                "tests": 187,
                "frames": 103,
            }
        )
    )


if __name__ == "__main__":
    main()
