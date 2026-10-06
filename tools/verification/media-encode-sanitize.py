"""Instrumented C encoding/AVIO failure paths over the exact patched component."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("media_check", Path(__file__).with_name("media-native-check.py"))
media = importlib.util.module_from_spec(spec)
spec.loader.exec_module(media)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--component", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--hardware", action="store_true")
    args = parser.parse_args()
    component, output = args.component.resolve(), args.output.resolve()
    record = media.verify_component(component, True)
    output.mkdir(parents=True, exist_ok=False)
    lib = component / "install/lib"
    probe = output / "encode-probe"
    env = dict(os.environ, ASAN_OPTIONS="halt_on_error=1", UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    commands = []

    def run(command, name):
        commands.append(list(map(str, command)))
        result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, timeout=180)
        (output / (name + ".stdout")).write_text(result.stdout)
        (output / (name + ".stderr")).write_text(result.stderr)
        result.check_returncode()
        return result.stdout

    run(["clang", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-g",
         "-I" + str(ROOT / "components/ffmpeg"), str(ROOT / "tools/verification/media-encode-probe.c"),
         "-L" + str(lib), "-Wl,-rpath," + str(lib), "-lmo_media", "-lm", "-o", str(probe)], "probe-build")
    checks = [json.loads(run([str(probe), str(output / "lossless.mp4")], "lossless"))]
    if args.hardware:
        checks.append(json.loads(run([str(probe), str(output / "h264.mp4"), "hardware"], "hardware")))
    report = {"status": "passed", "component": record, "checks": checks, "commands": commands,
              "scope": "C wrapper/upstream C ASan/UBSan on finite encoding, candidate short writes, failure/limits, internal cancellation. Platform frameworks and assembly are not instrumented; hard all-allocation bounds still require worker isolation."}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"status": "passed", "checks": checks}))


if __name__ == "__main__":
    main()
