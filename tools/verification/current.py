#!/usr/bin/env python3
"""Run the current repository gates with explicit tools and isolated output.

Historical stage drivers and evidence stay immutable. This runner never edits
or executes rewritten source. A missing prerequisite is a failed gate, not a
successful skip. External application/product acceptance remains separate.
"""
import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
GROUPS = ("lint", "rust", "contracts", "typescript", "native", "mcp", "wasm", "mcp-protocol")


@dataclass(frozen=True)
class Step:
    name: str
    argv: tuple[str, ...]
    worker: bool = False


def selected_groups(value):
    groups = set(value.split(","))
    if unknown := groups.difference(GROUPS):
        raise ValueError(f"unknown verification groups: {sorted(unknown)}")
    if "mcp-protocol" in groups:
        groups.update(("native", "mcp"))
    return groups


def plan(groups, output, bindgen, python):
    steps = []

    def add(name, *argv, worker=False):
        steps.append(Step(name, tuple(map(str, argv)), worker))

    cargo = ("--locked", "--offline")
    mcp = ("--manifest-path", "tools/mo-mcp/Cargo.toml")
    if "lint" in groups:
        add("verification-driver-tests", python, "-m", "unittest", "discover", "-s", "tools/verification", "-p", "test_current.py", "-v")
        add("agent-package-tests", python, "-m", "unittest", "discover", "-s", "tools/agent-package", "-p", "test_build.py", "-v")
        add("playback-sdk-package-tests", python, "-m", "unittest", "discover", "-s", "tools/playback-sdk", "-p", "test_build.py", "-v")
        add("format", "cargo", "fmt", "--all", "--", "--check")
        add("clippy", "cargo", "clippy", "--workspace", "--all-targets", *cargo, "--", "-D", "warnings")
    if "rust" in groups:
        add("rust-tests", "cargo", "test", "--workspace", *cargo)
    if "contracts" in groups:
        add("schemas", "cargo", "run", "-p", "mo-contract-codegen", *cargo, "--", "check", "contracts/generated")
    if "typescript" in groups:
        add("types", "pnpm", "check:types")
        add("client-tests", "pnpm", "test:operation-client")
        add("playback-client-tests", "pnpm", "test:playback-client")
        add("editor-client-tests", "pnpm", "test:editor-client")
    if "native" in groups or "wasm" in groups:
        add("native-build", "cargo", "build", *cargo, "-p", "mo-cli", "-p", "mo-host", "-p", "mo-raster-worker", "-p", "mo-export-worker")
    if "native" in groups:
        add("native-render-tests", "cargo", "test", *cargo, "-p", "mo-native-render", "--test", "delivery", "--", "--ignored", worker=True)
        add("native-playback-tests", "cargo", "test", *cargo, "-p", "mo-native-render", "--test", "playback", "--", "--ignored", worker=True)
        add("native-host-export-tests", "cargo", "test", *cargo, "-p", "mo-standard-host", "--test", "exports", "--", "--ignored", worker=True)
        add("native-export-tests", "cargo", "test", *cargo, "-p", "mo-export-worker", worker=True)
        add("native-computation-cli-tests", "cargo", "test", *cargo, "-p", "mo-cli", "--test", "computation", "--", "--ignored", worker=True)
    if "mcp" in groups:
        add("mcp-format", "cargo", "fmt", *mcp, "--", "--check")
        add("mcp-clippy", "cargo", "clippy", *mcp, "--all-targets", *cargo, "--", "-D", "warnings")
        add("mcp-tests", "cargo", "test", *mcp, *cargo)
        add("mcp-build", "cargo", "build", *mcp, *cargo)
        add("mcp-legacy-clippy", "cargo", "clippy", *mcp, "--features", "legacy-host", "--all-targets", *cargo, "--", "-D", "warnings")
        add("mcp-legacy-tests", "cargo", "test", *mcp, "--features", "legacy-host", *cargo)
        add("mcp-legacy-build", "cargo", "build", *mcp, "--features", "legacy-host", "--bin", "mo-mcp-legacy", *cargo)
        add("mcp-http-clippy", "cargo", "clippy", *mcp, "--features", "http", "--all-targets", *cargo, "--", "-D", "warnings")
        add("mcp-http-tests", "cargo", "test", *mcp, "--features", "http", *cargo)
        add("mcp-http-build", "cargo", "build", *mcp, "--features", "http", "--bin", "mo-mcp-http", *cargo)
        add("mcp-all-features-clippy", "cargo", "clippy", *mcp, "--all-features", "--all-targets", *cargo, "--", "-D", "warnings")
    if "wasm" in groups:
        add("pure-operation-wasm", "cargo", "check", *cargo, "-p", "mo-presentation-operations", "--target", "wasm32-unknown-unknown")
        add("legacy-host-wasm", "cargo", "check", *cargo, "-p", "mo-host-compat-wasm", "--target", "wasm32-unknown-unknown")
        add("wasm-build", "cargo", "build", *cargo, "--release", "-p", "mo-wasm", "--target", "wasm32-unknown-unknown")
        add("wasm-bindgen", bindgen, "target/wasm32-unknown-unknown/release/mo_wasm.wasm", "--target", "nodejs", "--out-dir", output / "wasm")
        add("document-native-wasm-parity", "node", "tools/verification/native-wasm-parity.mjs", "target/debug/mo-cli", output / "wasm/mo_wasm.js")
    if "mcp-protocol" in groups:
        thin = ROOT / "tools/mo-mcp/target/debug/mo-mcp"
        add("mcp-agent-package", python, "tools/agent-package/check.py", output / "mcp-agent-package", thin, ROOT / "target/debug/mo-export-worker")
        add("mcp-computation", python, "tools/mo-mcp/compute_check.py", output / "mcp-computation", thin, ROOT / "target/debug/mo-export-worker")
        add("mcp-templates", python, "tools/mo-mcp/template_check.py", output / "mcp-templates", thin, ROOT / "target/debug/mo-export-worker")
        if "wasm" in groups:
            add("template-protocol-parity", "node", "tools/verification/template-protocol-parity.mjs", output / "mcp-templates/2026-07-28/parity.json", ROOT / "target/debug/mo-cli", output / "wasm/mo_wasm.js", output / "template-protocol-parity")
        add("mcp-computation-lifecycle", python, "tools/mo-mcp/compute_lifecycle.py", output / "mcp-computation-lifecycle", thin)
        http = ROOT / "tools/mo-mcp/target/debug/mo-mcp-http"
        add("mcp-http-protocol", python, "tools/mo-mcp/http_check.py", output / "mcp-http-protocol", http, ROOT / "target/debug/mo-export-worker")
        add("mcp-http-lifecycle", python, "tools/mo-mcp/http_lifecycle.py", output / "mcp-http-lifecycle", http)
        binary = ROOT / "tools/mo-mcp/target/debug/mo-mcp-legacy"
        add("mcp-protocol", python, "tools/mo-mcp/check.py", output / "mcp-protocol", binary, ROOT / "target/debug/mo-raster-worker")
        for suite in ("transport", "lifecycle", "cancellation"):
            add(f"mcp-{suite}", python, f"tools/mo-mcp/check_{suite}.py", output / f"mcp-{suite}", binary)
    return steps


def file_digest(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as source:
        while block := source.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def source_identity():
    names = subprocess.check_output([
        "git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--",
        "crates", "tools", "packages", "contracts", "components", "fixtures", "integrations",
        "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "package.json", "pnpm-lock.yaml",
    ], cwd=ROOT).decode().split("\0")
    entries = [(name, file_digest(ROOT / name) if (ROOT / name).is_file() else None)
               for name in sorted(set(names).difference({""}))]
    return hashlib.sha256(json.dumps(entries, separators=(",", ":")).encode()).hexdigest()


def prerequisites(groups, args):
    errors = []
    for name in ("cargo", "git", *( ("pnpm", "node") if "typescript" in groups else ()), *( ("node",) if "wasm" in groups else ())):
        if not shutil.which(name):
            errors.append(f"executable unavailable: {name}")
    if groups.intersection(("lint", "rust", "native", "wasm")):
        for name, value in (("--skia-dir", args.skia_dir), ("--harfbuzz-dir", args.harfbuzz_dir)):
            if not value or not Path(value).is_dir():
                errors.append(f"{name} must identify a prepared component directory; see component build documentation")
    if "wasm" in groups and (not args.bindgen or not shutil.which(args.bindgen)):
        errors.append("--bindgen must identify a compatible wasm-bindgen executable")
    if "mcp-protocol" in groups and importlib.util.find_spec("jsonschema") is None:
        errors.append("the runner Python environment requires jsonschema for MCP protocol checks")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--groups", default=",".join(GROUPS))
    parser.add_argument("--output", type=Path, required=True, help="new output directory; existing runs are never overwritten")
    parser.add_argument("--skia-dir", default=os.getenv("MO_SKIA_LIB_DIR"))
    parser.add_argument("--harfbuzz-dir", default=os.getenv("MO_HARFBUZZ_LIB_DIR"))
    parser.add_argument("--bindgen", default=os.getenv("MO_WASM_BINDGEN_BIN"))
    parser.add_argument("--list", action="store_true", help="print the selected gate plan without running it")
    args = parser.parse_args()
    try:
        groups = selected_groups(args.groups)
    except ValueError as error:
        parser.error(str(error))
    output = args.output.resolve()
    steps = plan(groups, output, args.bindgen or "wasm-bindgen", sys.executable)
    if args.list:
        for step in steps:
            print(json.dumps({"name": step.name, "argv": step.argv}))
        return 0
    output.mkdir(parents=True, exist_ok=False)
    report = {"format": "musteroffice.current-verification/1", "startedAt": datetime.now(timezone.utc).isoformat(),
              "selectedGroups": sorted(groups), "notSelected": sorted(set(GROUPS) - groups),
              "scope": "Current code gates; document Native/WASM parity. No full rendering parity, Office/WPS, product acceptance or release performance claim.",
              "steps": [{"name": step.name, "argv": step.argv, "status": "not-run"} for step in steps]}

    def save():
        temporary = output / "report.tmp"
        temporary.write_text(json.dumps(report, indent=2) + "\n")
        temporary.replace(output / "report.json")

    report["prerequisiteErrors"] = prerequisites(groups, args)
    if report["prerequisiteErrors"]:
        report["status"] = "blocked"
        save()
        print(json.dumps(report["prerequisiteErrors"]), flush=True)
        return 2
    report["head"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    report["sourceSha256"] = source_identity()
    env = dict(os.environ)
    # Commands and worker paths use this repository's known output locations.
    env.pop("CARGO_TARGET_DIR", None)
    for key, value in (("MO_SKIA_LIB_DIR", args.skia_dir), ("MO_HARFBUZZ_LIB_DIR", args.harfbuzz_dir)):
        if value:
            env[key] = str(Path(value).resolve())
    report["status"] = "running"
    save()
    for step, record in zip(steps, report["steps"]):
        log = output / f"{step.name}.log"
        record.update(status="running", log=str(log))
        save()
        print(json.dumps({"step": step.name, "status": "running"}), flush=True)
        start = time.monotonic()
        try:
            if step.worker:
                for stem, key in (("mo-raster-worker", "MO_DELIVERY_WORKER"), ("mo-export-worker", "MO_EXPORT_WORKER_TEST_BIN")):
                    path = ROOT / "target/debug" / stem
                    sha = file_digest(path)
                    env[key] = str(path)
                    env["MO_DELIVERY_WORKER_SHA256" if stem == "mo-raster-worker" else "MO_EXPORT_WORKER_TEST_SHA256"] = sha
                    record.setdefault("workers", {})[stem] = {"path": str(path), "sha256": sha}
                save()
            with log.open("x") as stream:
                result = subprocess.run(step.argv, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=1800)
            code = result.returncode
            if code == 0 and step.name == "wasm-bindgen":
                (output / "wasm/package.json").write_text('{"private":true,"type":"commonjs"}\n')
        except (OSError, subprocess.TimeoutExpired) as error:
            record["error"] = str(error)
            code = 1
        record.update(status="passed" if code == 0 else "failed", exitCode=code, durationSeconds=time.monotonic() - start)
        save()
        print(json.dumps({"step": step.name, "status": record["status"]}), flush=True)
        if code:
            break
    report["finalSourceSha256"] = source_identity()
    report["status"] = "passed" if all(s["status"] == "passed" for s in report["steps"]) and report["sourceSha256"] == report["finalSourceSha256"] else "failed"
    report["finishedAt"] = datetime.now(timezone.utc).isoformat()
    save()
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
