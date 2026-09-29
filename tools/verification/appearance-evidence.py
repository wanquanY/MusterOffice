"""Verify bounded native appearance and isolated product integration evidence."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--product-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    core = Path(__file__).resolve().parents[2]
    product = args.product_root.resolve()
    stage = core / ".codex-work/appear-native"
    product_stage = product / ".codex-work/integration/appearance-native"
    files = {}

    def label(path):
        path = path.resolve()
        for root, prefix in [(core, "MusterOffice:"), (product, "Musterwork:")]:
            if path.is_relative_to(root):
                return prefix + path.relative_to(root).as_posix()
        raise ValueError(f"not an owned input: {path}")

    def include(path, digest=None):
        path = path.resolve()
        actual = sha(path)
        assert digest is None or digest == actual, label(path)
        files[label(path)] = actual
        return {
            "path": label(path),
            "sha256": actual,
            "byteLength": path.stat().st_size,
        }

    # One additional active-group rollback test was added after the broad run.
    # Its final bytes were compiled/tested and linted; no production source changed.
    extra_test = core / "crates/mo-pptx/tests/timing_appearance.rs"
    final_test_hash = read(stage / "active-reverse-01.json")["sourceAfter"][
        extra_test.relative_to(core).as_posix()
    ]
    include(extra_test, final_test_hash)
    commands, test_updates = [], set()
    core_names = [
        "rust-01",
        "native-build-01",
        "wasm-build-01",
        "lint-01",
        "corpus-03",
        "sdk-build-01",
        "export-worker-build-01",
        "wasm-bindgen-01",
        "schema-01",
        "bundle-01",
        "wasm-sync-01",
        "wasm-stepped-01",
        "wasm-visibility-01",
        "wasm-legacy-01",
        "wasm-seek-01",
        "identity-audit-01",
        "wasm-navigation-02",
        "active-reverse-01",
        "active-reverse-lint-01",
        "types-01",
        "python-lint-01",
    ]
    product_names = [
        "prepare-sdk-01",
        "prepare-playback-02",
        "consumer-build-01",
        "adapter-01",
        "preparation-01",
        "client-01",
        "viewer-01",
        "types-01",
        "desktop-build-01",
    ]
    for root, folder, names in [
        (core, stage, core_names),
        (product, product_stage, product_names),
    ]:
        for name in names:
            report = folder / (name + ".json")
            value = read(report)
            assert value["exitCode"] == 0 and value["sourceUnchanged"], name
            assert value["sourceBefore"] == value["sourceAfter"], name
            for source, digest in value["sourceAfter"].items():
                path = (root / source).resolve()
                if path == extra_test and digest != final_test_hash:
                    test_updates.add(digest)
                else:
                    include(path, digest)
            commands.append(
                {
                    "report": include(report),
                    "log": include(folder / (name + ".log"), value["logSha256"]),
                    "exitCode": 0,
                    "sourceUnchangedDuringCommand": True,
                }
            )
    failures = []
    for folder, names in [
        (stage, ["corpus-01", "corpus-02", "wasm-navigation-01"]),
        (product_stage, ["prepare-playback-01"]),
    ]:
        for name in names:
            value = read(folder / (name + ".json"))
            assert value["exitCode"] != 0
            failures.append(
                {
                    "report": include(folder / (name + ".json")),
                    "log": include(folder / (name + ".log"), value["logSha256"]),
                }
            )

    # SDK manifests, complete inventories, current product pins and emitted assets.
    lock = read(product / "components/musteroffice/lock.json")
    playback = read(product / "components/musteroffice/playback-lock.json")[
        "manifestSha256"
    ]
    sdk = product / "apps/agent-runtime/vendor/musteroffice" / lock["sdkManifestSha256"]
    bundle = product / "components/musteroffice/generated/playback" / playback
    for directory, manifest, pin in [
        (sdk, "sdk-manifest.json", lock["sdkManifestSha256"]),
        (bundle, "bundle-manifest.json", playback),
    ]:
        include(directory / manifest, pin)
        entries = read(directory / manifest)["files"]
        expected = {entry["path"] for entry in entries} | {manifest}
        assert {
            p.relative_to(directory).as_posix()
            for p in directory.rglob("*")
            if p.is_file()
        } == expected
        for entry in entries:
            assert (
                include(directory / entry["path"], entry["sha256"])["byteLength"]
                == entry["byteLength"]
            )
    worker_pin = lock["workers"]["darwin-arm64"]
    worker = (
        product
        / "components/musteroffice/generated/workers/darwin-arm64"
        / worker_pin["sha256"]
        / "mo-export-worker"
    )
    assert (
        include(worker, worker_pin["sha256"])["byteLength"] == worker_pin["byteLength"]
    )
    emitted = read(product_stage / "emitted-01.json")
    emitted_root = product_stage / "desktop-build-01/musteroffice" / playback
    for entry in emitted["files"]:
        assert (
            include(emitted_root / entry["path"], entry["sha256"])["byteLength"]
            == entry["byteLength"]
        )
    include(product_stage / "emitted-01.json")

    frames = {}
    for name in [
        "wasm-sync-01",
        "wasm-stepped-01",
        "wasm-visibility-01",
        "wasm-legacy-01",
        "wasm-seek-01",
        "wasm-navigation-02",
    ]:
        value = read(stage / name / "report.json")
        assert value["status"] == "passed" and value["bundleManifestSha256"] == playback
        for path, digest in value["inputs"].items():
            include(core / path, digest)
        for path in (stage / name).iterdir():
            if path.is_file():
                include(path)
        frames[name] = value["frameCount"]
    assert list(frames.values()) == [78, 78, 54, 259, 130, 60]
    for name in ["corpus-03", "navigation-identity-01"]:
        for path in (stage / name).iterdir():
            if path.is_file():
                include(path)
    for path in (product_stage / "fixtures-02").rglob("*"):
        if path.is_file():
            include(path)
    for path, expected in read(product_stage / "fixtures-02/report.json")[
        "inputs"
    ].items():
        include(Path(path), expected["sha256"])
    observed = read(stage / "wps-observations.json")
    for entry in observed["inputs"]:
        include(core / entry["path"], entry["sha256"])
    for observation in observed["observations"]:
        for entry in observation["screenshots"]:
            include(core / entry["path"], entry["sha256"])
    include(stage / "wps-observations.json")
    xsd = read(stage / "xsd.json")
    assert xsd["presentationParts"] == 10 and xsd["packages"] == 2
    include(stage / "xsd.json")
    for path in (core / ".codex-work/ecma376/xsd").rglob("*.xsd"):
        include(path)
    actual = read(product_stage / "actual-worker-01.json")
    assert actual["frames"] == 38
    include(product_stage / "actual-worker-01.json")
    for case in actual["cases"]:
        folder = product_stage / "fixtures-02" / case["name"]
        include(folder / "request.json", case["requestSha256"])
        include(folder / "contents.bin", case["packetSha256"])
    totals = [
        tuple(map(int, value))
        for value in re.findall(
            r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored",
            (stage / "rust-01.log").read_text(),
        )
    ]
    assert [sum(v[i] for v in totals) for i in range(3)] == [554, 0, 9]
    include(Path(__file__))
    value = {
        "format": "musteroffice.native-appearance-integration-verification/1",
        "status": "passed",
        "fullGoal": "active",
        "replacementAccepted": False,
        "core": {
            "rustTests": 555,
            "ordinaryIgnored": 9,
            "newNativeFrames": 78,
            "staticControls": 4,
            "wasmFrames": frames,
            "oldIdentityOnlyChanges": 6,
            "xsdParts": 10,
        },
        "product": {
            "tests": 158,
            "actualWorkerFrames": 46,
            "newDeliveryFrames": 38,
            "previousPublishedDeliveryFrames": 8,
            "newDeliveriesCommitted": False,
            "sdkManifestSha256": lock["sdkManifestSha256"],
            "exportWorker": worker_pin,
            "playbackManifestSha256": playback,
            "emittedFiles": emitted["fileCount"],
            "emittedUncompressedBytes": emitted["uncompressedBytes"],
            "emittedDeltaBytes": emitted["deltaBytes"],
        },
        "wps": {
            "version": observed["version"],
            "observations": include(stage / "wps-observations.json"),
        },
        "additionalTest": {
            "path": label(extra_test),
            "beforeDigests": sorted(test_updates),
            "verifiedDigest": final_test_hash,
            "reason": "Active-group rollback test added after broad suite; targeted test and strict lint passed. No production source changed.",
        },
        "commands": commands,
        "retainedFailures": failures,
        "verifiedFiles": dict(sorted(files.items())),
        "limitations": [
            "Single Appear/Disappear native presets and main-sequence rollback, not all animation semantics.",
            "New product deliveries validate computation and real product Worker; no new database publication or model session.",
            "WPS observations are discrete playback states; no Microsoft Office, WPS edit/save roundtrip or multi-effect acceptance.",
            "Other advanced content, full Electron/history migration, performance, installer size and complete replacement gates remain open.",
        ],
    }
    with args.output.open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")
    print(
        json.dumps(
            {"status": "passed", "files": len(files), "sha256": sha(args.output)}
        )
    )


if __name__ == "__main__":
    main()
