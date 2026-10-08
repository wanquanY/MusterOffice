"""Link our shared AAC/MP4 core into a bounded, callback-only ES module.

Consumes a verified, fixed library build; keeps relinkable objects and licenses.
Output is an immutable development component, never a public release.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
COMPONENT = ROOT / "components/ffmpeg"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(base, paths):
    return [{"path": str(p.relative_to(base)), "sha256": sha(p), "byteLength": p.stat().st_size} for p in sorted(paths)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--libraries", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--emsdk", type=Path, default=ROOT / ".codex-work/emsdk")
    args = parser.parse_args()
    libraries, output, emsdk = args.libraries.resolve(), args.output.resolve(), args.emsdk.resolve()
    if output.is_relative_to(ROOT) and not output.is_relative_to(ROOT / ".codex-work"):
        raise ValueError("output must not replace repository sources")
    record = json.loads((libraries / "libraries.json").read_text())
    lock = json.loads((COMPONENT / "lock.json").read_text())
    tool_lock = json.loads((ROOT / "components/skia/lock.json").read_text())
    if record["format"] != "musteroffice.browser-encode-libraries/1-draft" or record["source"] != lock:
        raise ValueError("library source/profile mismatch")
    if record["emscripten"]["tools"] != tool_lock["emsdkTools"] or record["emscripten"]["version"] != tool_lock["emsdkVersion"]:
        raise ValueError("compiler profile mismatch")
    for base, rows in [(libraries, record["artifacts"] + record["headers"] + record["configuration"]), (emsdk, tool_lock["emsdkTools"])]:
        for row in rows:
            path = base / row["path"]
            if sha(path) != row["sha256"] or path.stat().st_size != row["byteLength"]:
                raise ValueError("build input changed: " + row["path"])
    configuration = libraries / "source" / ("ffmpeg-" + lock["version"]) / "config_components.h"
    enabled = sorted(re.findall(r"^#define CONFIG_(\w+(?:ENCODER|DECODER|PARSER|MUXER|DEMUXER|PROTOCOL)) 1$", configuration.read_text(), re.M))
    if enabled != ["AAC_ENCODER", "AC3_PARSER", "H264_PARSER", "MOV_MUXER", "MP4_MUXER"]:
        raise ValueError("unexpected browser codec/container dependency closure")
    output.mkdir(parents=True, exist_ok=False)
    (output / "relink").mkdir()
    (output / "runtime").mkdir()
    sources = [COMPONENT / p for p in ["mo_media.c", "mo_encode.c", "mo_encode_mux.c", "mo_browser_encode.c"]]
    headers = [COMPONENT / p for p in ["mo_media.h", "mo_media_internal.h", "mo_encode.h", "mo_encode_internal.h"]]
    compiler = emsdk / "upstream/emscripten/emcc"
    env = {**os.environ, "EM_CONFIG": str(emsdk / ".emscripten")}
    commands = []

    def run(command):
        commands.append(list(map(str, command)))
        with (output / "build.log").open("a") as log:
            subprocess.run(command, env=env, check=True, stdout=log, stderr=log)

    objects = []
    for source in sources:
        obj = output / "relink" / (source.stem + ".o")
        run([str(compiler), "-std=c11", "-O3", "-msimd128", "-ffp-contract=off", "-fvisibility=hidden",
             "-Wall", "-Wextra", "-Werror", "-I" + str(libraries / "install/include"), "-I" + str(COMPONENT),
             "-c", str(source), "-o", str(obj)])
        objects.append(obj)
    exports = ["malloc", "free", "mo_browser_layout", "mo_browser_create", "mo_browser_config", "mo_browser_limits",
               "mo_browser_error", "mo_browser_stats", "mo_browser_open", "mo_browser_video", "mo_browser_audio",
               "mo_browser_finish", "mo_browser_close"]
    archives = [libraries / "install/lib" / ("lib" + name + ".a") for name in ["avformat", "avcodec", "avutil"]]
    run([str(compiler), "-O3", "-msimd128", *map(str, objects), *map(str, archives),
         "--no-entry", "-sMODULARIZE=1", "-sEXPORT_ES6=1", "-sENVIRONMENT=worker", "-sFILESYSTEM=0",
         "-sALLOW_MEMORY_GROWTH=1", "-sINITIAL_MEMORY=16777216", "-sMAXIMUM_MEMORY=268435456",
         "-sABORTING_MALLOC=0", "-sDYNAMIC_EXECUTION=0", "-sSTACK_SIZE=1048576",
         "-sEXPORTED_FUNCTIONS=" + json.dumps(["_" + p for p in exports]),
         "-sEXPORTED_RUNTIME_METHODS=" + json.dumps(["HEAPU8"]),
         "-o", str(output / "runtime/encode.mjs")])
    for path in archives:
        shutil.copyfile(path, output / "relink" / path.name)
    for name in ["COPYING.LGPLv2.1", "LICENSE.md"]:
        shutil.copyfile(libraries / name, output / name)
    for name in ["LICENSE", "NOTICE"]:
        shutil.copyfile(ROOT / name, output / name)
    (output / "toolchain-licenses").mkdir()
    for source, name in [("LICENSE", "emscripten.txt"), ("system/lib/libc/musl/COPYRIGHT", "musl.txt"),
                         ("system/lib/compiler-rt/LICENSE.TXT", "compiler-rt.txt")]:
        shutil.copyfile(emsdk / "upstream/emscripten" / source, output / "toolchain-licenses" / name)
    wrapper = ROOT / "packages/video-browser-encode/src/index.mjs"
    project = ROOT / "packages/video-browser-encode/src/project.mjs"
    shutil.copyfile(wrapper, output / "index.mjs")
    shutil.copyfile(project, output / "project.mjs")
    declarations = [wrapper.with_suffix('.d.ts'), project.with_suffix('.d.ts')]
    for path in declarations:
        shutil.copyfile(path, output / path.name.replace('.d.ts', '.d.mts'))
    wasm = output / "runtime/encode.wasm"
    manifest = {"format": "musteroffice.browser-encode-component/1-draft", "releaseCleared": False,
                "sourceCommit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "sourceDirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip()),
                "libraryRecordSha256": sha(libraries / "libraries.json"), "libraries": record, "enabledComponents": enabled,
                "sources": inventory(ROOT, sources + headers + declarations + [wrapper, project, Path(__file__).resolve(),
                    ROOT / "tools/components/build-browser-encode.py"]),
                "commands": commands, "wasmGzipBytes": len(gzip.compress(wasm.read_bytes(), mtime=0)),
                "files": inventory(output, [p for p in output.rglob("*") if p.is_file() and p.name != "build.log"])}
    (output / "bundle-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"status": "built", "wasmBytes": wasm.stat().st_size, "wasmGzipBytes": manifest["wasmGzipBytes"]}))


if __name__ == "__main__":
    main()
