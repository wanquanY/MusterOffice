"""Check source-bound visibility evidence and emit a bounded stage report."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    stage = args.stage
    paths = {}

    def include(path, expected=None):
        digest = sha(path)
        require(expected is None or digest == expected, f"changed input: {path}")
        key = path.as_posix()
        require(paths.get(key, digest) == digest, f"inconsistent input: {path}")
        paths[key] = digest
        return digest

    commands = []
    names = [
        "native-build-02",
        "native-03",
        "rust-01",
        "lint-01",
        "wasm-build-02",
        "bundle-01",
        "schema-check-01",
        "types-check-01",
        "corpus-03",
        "wasm-parity-01",
        "wasm-sync-01",
        "wasm-legacy-01",
        "wasm-navigation-01",
        "wasm-seek-01",
        "python-lint-01",
    ]
    for name in names:
        report = stage / (name + ".json")
        value = read(report)
        require(value["exitCode"] == 0 and value["sourceUnchanged"], name)
        require(value["sourceBefore"] == value["sourceAfter"], name)
        for path, digest in value["sourceAfter"].items():
            include(Path(path), digest)
        include(stage / (name + ".log"), value["logSha256"])
        commands.append(
            {
                "report": report.as_posix(),
                "sha256": include(report),
                "args": [
                    arg.replace(str(Path.cwd()), "${REPOSITORY}")
                    for arg in value["args"]
                ],
                "exitCode": 0,
                "sourceUnchanged": True,
            }
        )
    # The final pinned worker reran the earlier reference corpus. Require every
    # input, metadata and pixel byte to agree, not just selected pixel hashes.
    for path in (stage / "native-02").rglob("*"):
        if path.is_file():
            later = stage / "native-03" / path.relative_to(stage / "native-02")
            include(later, include(path))
    raw = (stage / "rust-01.log").read_text()
    totals = [
        tuple(map(int, match))
        for match in re.findall(
            r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", raw
        )
    ]
    totals = list(map(sum, zip(*totals, strict=True)))
    require(totals == [547, 0, 9], "unexpected Rust test totals")
    counts = {}
    for name in [
        "wasm-parity-01",
        "wasm-sync-01",
        "wasm-legacy-01",
        "wasm-navigation-01",
        "wasm-seek-01",
    ]:
        value = read(stage / name / "report.json")
        require(value["status"] == "passed", name)
        for path, digest in value["inputs"].items():
            include(Path(path), digest)
        counts[name] = value["frameCount"]
        for path in (stage / name).iterdir():
            if path.is_file():
                include(path)
    require(list(counts.values()) == [54, 54, 259, 60, 130], "unexpected frame counts")
    structure = read(stage / "format-02/report.json")
    require(
        structure["status"] == "passed" and structure["xsdParts"] == 45,
        "native schema validation",
    )
    require(len(structure["cliPixelComparisons"]) == 7, "CLI regression coverage")
    for entry in structure["files"]:
        include(Path(entry["file"]), entry["sha256"])
    for path in (stage / "format-02").iterdir():
        if path.is_file():
            include(path)
    for path in Path(".codex-work/ecma376/xsd").rglob("*.xsd"):
        include(path)
    include(stage / "format-and-cli-02.py")
    for path in [
        Path(__file__).relative_to(Path.cwd())
        if Path(__file__).is_absolute()
        else Path(__file__),
        Path("tools/verification/visibility-fixtures.py"),
    ]:
        include(path)
    report = {
        "format": "musteroffice.visibility-set-verification/1",
        "status": "passed",
        "fullGoal": "active",
        "replacementAccepted": False,
        "rust": {
            "passed": totals[0],
            "ignoredInOrdinaryRun": totals[2],
            "additionalPinnedWorkerTestPassed": 1,
        },
        "samples": {
            "newNativeAuthor": 9,
            "newNativeSource": 45,
            "newWasmSync": 54,
            "newWasmStepped": 54,
            "previousFrozenWasmSamples": 449,
            "cliPixelComparisons": 7,
        },
        "xsd": {"presentationParts": 45, "documents": 6},
        "bundleManifestSha256": include(stage / "bundle-01/bundle-manifest.json"),
        "workerSha256": include(Path("target/debug/mo-raster-worker")),
        "commands": commands,
        "verifiedFiles": dict(sorted(paths.items())),
        "limitations": [
            "Generic fixed visibility assignment; entrance/exit presets remain unsupported.",
            "Independent static controls and shared-kernel Native/WASM; no new Office/WPS observation.",
            "Product pins unchanged; complete advanced content and replacement acceptance remain open.",
            "No installer, latency or peak-memory measurement is claimed.",
        ],
    }
    with args.output.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")
    print(
        json.dumps(
            {
                "status": "passed",
                "verifiedFiles": len(paths),
                "reportSha256": sha(args.output),
            }
        )
    )


if __name__ == "__main__":
    main()
