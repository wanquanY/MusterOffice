#!/usr/bin/env python3
"""Build pinned, source-verified portable DSP; never fetch or use system DSP."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from native_platform import native_platform

ROOT = Path(__file__).resolve().parents[2]
COMPONENT = ROOT / "components/signalsmith-stretch"


def record(path, base):
    data = path.read_bytes()
    return {"path": str(path.relative_to(base)), "byteLength": len(data),
            "sha256": hashlib.sha256(data).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=("native", "wasm"), required=True)
    parser.add_argument("--stretch-source", type=Path, required=True)
    parser.add_argument("--linear-source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--emsdk", type=Path, default=ROOT / ".codex-work/emsdk")
    parser.add_argument("--sanitize", action="store_true")
    parser.add_argument("--fault-tests", action="store_true")
    parser.add_argument("--archiver", default="llvm-ar")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    lock = json.loads((COMPONENT / "lock.json").read_text())
    source = output / "source"
    transformations = []
    for entry in lock["sources"]:
        upstream = getattr(args, entry["name"] + "_source").resolve()
        dest = source if entry["name"] == "stretch" else source / "signalsmith-linear"
        dest.mkdir(parents=True, exist_ok=True)
        for row in entry["files"]:
            data = (upstream / row["path"]).read_bytes()
            assert len(data) == row["byteLength"] and hashlib.sha256(data).hexdigest() == row["sha256"], row
            if row["path"].endswith(".h"):
                # Auditable integration-only substitutions, not silent forks of
                # the spectral algorithm. All selected dynamic vectors use an
                # instance allocator; distribution mapping is fixed across STL.
                assert data.count(b"std::vector<") == row["vectorReplacements"]
                data = data.replace(b"std::vector<", b"mo_stretch::Vector<")
                if entry["name"] == "stretch":
                    assert data.count(b"std::uniform_real_distribution<Sample>") == 1
                    data = data.replace(b"std::uniform_real_distribution<Sample>", b"mo_stretch::UniformReal<Sample>")
            target = dest / row["path"]
            target.write_bytes(data)
            transformations.append(record(target, output))
    compiler = "clang++" if args.target == "native" else str(args.emsdk.resolve() / "upstream/emscripten/em++")
    env = os.environ.copy()
    if args.target == "wasm":
        env["EM_CONFIG"] = str(args.emsdk.resolve() / ".emscripten")
    common = ["-std=c++17", "-O2", "-DNDEBUG", "-fno-rtti", "-ffp-contract=off", "-fno-fast-math",
              "-I", str(source), "-I", str(COMPONENT)]
    if args.fault_tests:
        common.append("-DMO_STRETCH_FAULT_TEST")
    if args.sanitize:
        assert args.target == "native"
        common.extend(["-g1", "-fno-omit-frame-pointer", "-fsanitize=address,undefined"])
    commands = []

    def run(command):
        commands.append(command)
        with (output / "build.log").open("a") as log:
            subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=log, check=True)

    if args.target == "native":
        obj = output / "mo_stretch.o"
        run([compiler, *common, "-c", str(COMPONENT / "mo_stretch.cpp"), "-o", str(obj)])
        library = output / "libmo_stretch.a"
        library.unlink(missing_ok=True)
        run([args.archiver, "rcsD", str(library), str(obj)])
        artifacts = [library]
        if args.fault_tests:
            probe = output / "stretch-probe"
            run([compiler, *common, str(ROOT / "tools/verification/stretch-component-probe.cpp"),
                 str(library), "-o", str(probe)])
            artifacts.append(probe)
    else:
        exports = ["_mo_stretch_" + name for name in ("abi", "create", "destroy", "latency", "memory", "begin", "process")]
        exports += ["_malloc", "_free"]
        if args.fault_tests:
            exports.append("_mo_stretch_fail_after")
        module = output / "mo-stretch.mjs"
        run([compiler, *common, "-fwasm-exceptions", str(COMPONENT / "mo_stretch.cpp"), "--no-entry",
             "-sMODULARIZE=1", "-sEXPORT_ES6=1", "-sENVIRONMENT=node,web", "-sFILESYSTEM=0",
             "-sALLOW_MEMORY_GROWTH=1", "-sMAXIMUM_MEMORY=268435456", "-sSTACK_SIZE=1048576",
             "-sABORTING_MALLOC=0", "-sDYNAMIC_EXECUTION=0", "-sEXPORTED_FUNCTIONS=" + json.dumps(exports),
             '-sEXPORTED_RUNTIME_METHODS=["HEAPF32","HEAPU32"]', "-o", str(module)])
        artifacts = [module, module.with_suffix(".wasm")]
    result = {"format": "musteroffice.stretch-build/1", "target": args.target,
              "builder": record(Path(__file__).resolve(), ROOT),
              "lock": lock, "sanitizers": args.sanitize, "faultTests": args.fault_tests,
              "compiler": subprocess.check_output([compiler, "--version"], env=env, text=True),
              "sources": [record(COMPONENT / name, COMPONENT) for name in ("mo_stretch.h", "mo_stretch.cpp", "mo_stretch_memory.h")],
              "preparedSources": transformations, "commands": commands,
              "artifacts": [record(path, output) for path in artifacts]}
    if args.target == "native":
        result["nativePlatform"] = native_platform()
        result["archiver"] = subprocess.check_output([args.archiver, "--version"], text=True)
    if args.fault_tests:
        result["probeSource"] = record(ROOT / "tools/verification/stretch-component-probe.cpp", ROOT)
    (output / "build.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"target": args.target, "artifacts": result["artifacts"]}))


if __name__ == "__main__":
    main()
