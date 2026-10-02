"""Retain actual target, dependencies and test evidence beside a Linux worker."""
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import sys


def file_record(path):
    return {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "byteLength": path.stat().st_size}


output = Path(sys.argv[1])
if platform.system() != "Linux" or platform.machine() != "x86_64":
    raise RuntimeError("this release profile requires Linux x86_64 execution")
binary = output / "mo-export-worker"
header = binary.read_bytes()[:20]
if header[:6] != b"\x7fELF\x02\x01" or int.from_bytes(header[18:20], "little") != 62:
    raise RuntimeError("worker is not an x86_64 ELF executable")
if "not found" in (output / "dynamic-libraries.txt").read_text():
    raise RuntimeError("unresolved Linux runtime dependency")
report = {"format": "musteroffice.linux-worker-build/1", "target": "linux-x64",
          "worker": file_record(binary), "tests": file_record(output / "tests.log"),
          "dynamicLibraries": file_record(output / "dynamic-libraries.txt"),
          "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
          "components": {}}
for name, file in [("skia", "skia/native-build.json"), ("harfbuzz", "harfbuzz/native-build.json"),
                   ("imageCodecs", "codecs/build.json")]:
    value = json.loads((Path(".codex-work/linux") / file).read_text())
    if value["nativePlatform"] != {"os": "linux", "arch": "x86_64"}:
        raise RuntimeError("component platform differs from worker")
    (output / (name + "-build.json")).write_text(json.dumps(value, indent=2) + "\n")
    report["components"][name] = file_record(output / (name + "-build.json"))
(output / "build.json").write_text(json.dumps(report, indent=2) + "\n")
