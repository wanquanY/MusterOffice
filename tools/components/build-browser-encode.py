"""Build the fixed AAC/MP4-only browser leaf; no downloads or system codecs.

This does not build a software video codec or a virtual filesystem. The browser
supplies compressed AVC; shared Rust owns exact project scheduling.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
COMPONENT = ROOT / "components/ffmpeg"
LOCK = json.loads((COMPONENT / "lock.json").read_text())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(base, paths):
    return [{"path": str(p.relative_to(base)), "sha256": sha(p), "byteLength": p.stat().st_size} for p in sorted(paths)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--emsdk", type=Path, default=ROOT / ".codex-work/emsdk")
    parser.add_argument("--jobs", type=int, default=min(8, os.cpu_count() or 1))
    args = parser.parse_args()
    if not 1 <= args.jobs <= 32:
        parser.error("jobs must be 1..32")
    archive, output, emsdk = args.archive.resolve(), args.output.resolve(), args.emsdk.resolve()
    if output.is_relative_to(ROOT) and not output.is_relative_to(ROOT / ".codex-work"):
        raise ValueError("output must not replace repository sources")
    if archive.stat().st_size != LOCK["byteLength"] or sha(archive) != LOCK["sha256"]:
        raise ValueError("fixed FFmpeg source archive required")
    # Reuse the already recorded compiler coordinates, not Skia libraries or code.
    # Full installer archive hash is unavailable; this remains development-only.
    tool_lock = json.loads((ROOT / "components/skia/lock.json").read_text())
    if json.loads((emsdk / "upstream/emscripten/emscripten-version.txt").read_text()) != tool_lock["emsdkVersion"]:
        raise ValueError("Emscripten version mismatch")
    for row in tool_lock["emsdkTools"]:
        if sha(emsdk / row["path"]) != row["sha256"]:
            raise ValueError("Emscripten tool changed: " + row["path"])
    output.mkdir(parents=True, exist_ok=False)
    source = output / "source"
    source.mkdir()
    with tarfile.open(archive) as tar:
        tar.extractall(source, filter="data")
    upstream = source / ("ffmpeg-" + LOCK["version"])
    prefix = output / "install"
    env = os.environ.copy()
    env["EM_CONFIG"] = str(emsdk / ".emscripten")
    compiler = emsdk / "upstream/emscripten/emcc"
    tools = emsdk / "upstream/emscripten"
    commands = []

    def run(command):
        commands.append(list(map(str, command)))
        with (output / "build.log").open("a") as log:
            subprocess.run(command, cwd=upstream, env=env, check=True, stdout=log, stderr=log)

    flags = ["--prefix=" + str(prefix), "--cc=" + str(compiler), "--ar=" + str(tools / "emar"),
             "--ranlib=" + str(tools / "emranlib"), "--nm=" + str(emsdk / "upstream/bin/llvm-nm"),
             "--arch=wasm32", "--target-os=none", "--enable-cross-compile",
             "--disable-autodetect", "--disable-everything", "--disable-network", "--disable-protocols",
             "--disable-programs", "--disable-doc", "--disable-debug", "--disable-stripping",
             "--disable-gpl", "--disable-nonfree", "--disable-version3", "--enable-static", "--disable-shared",
             "--disable-avdevice", "--disable-avfilter", "--disable-swscale", "--disable-swresample",
             "--disable-pthreads", "--disable-w32threads", "--disable-os2threads",
             "--disable-runtime-cpudetect", "--enable-avformat", "--enable-avcodec", "--enable-avutil",
             "--enable-encoder=aac", "--enable-parser=h264", "--enable-muxer=mp4",
             "--enable-small", "--extra-cflags=-O3 -msimd128 -ffp-contract=off",
             "--extra-ldflags=-msimd128"]
    run([str(upstream / "configure"), *flags])
    config = (upstream / "config.h").read_text()
    for name in ["GPL", "GPLV3", "NONFREE", "VERSION3", "NETWORK"]:
        if f"#define CONFIG_{name} 0" not in config:
            raise ValueError("unexpected component configuration: " + name)
    components = (upstream / "config_components.h").read_text()
    enabled = sorted(re.findall(r"^#define CONFIG_(\w+(?:ENCODER|DECODER|PARSER|MUXER|DEMUXER|PROTOCOL)) 1$", components, re.M))
    if enabled != ["AAC_ENCODER", "AC3_PARSER", "H264_PARSER", "MOV_MUXER", "MP4_MUXER"]:
        raise ValueError("unexpected codec/container dependency closure: " + str(enabled))
    run(["make", "-j" + str(args.jobs)])
    run(["make", "install"])
    archives = list((prefix / "lib").glob("*.a"))
    if {p.name for p in archives} != {"libavformat.a", "libavcodec.a", "libavutil.a"}:
        raise ValueError("unexpected static library closure")
    for name in ("LICENSE.md", "COPYING.LGPLv2.1"):
        shutil.copyfile(upstream / name, output / name)
    headers = list((prefix / "include").rglob("*.h"))
    value = {"format": "musteroffice.browser-encode-libraries/1-draft", "releaseCleared": False,
             "source": LOCK, "modifiedUpstream": False, "commands": commands, "enabledComponents": enabled,
             "emscripten": {"version": tool_lock["emsdkVersion"], "tools": tool_lock["emsdkTools"],
                            "limitation": tool_lock["emsdkToolchainLimitation"]},
             "artifacts": inventory(output, archives), "headers": inventory(output, headers),
             "configuration": inventory(output, [upstream / "config.h", upstream / "config_components.h", upstream / "ffbuild/config.mak"])}
    (output / "libraries.json").write_text(json.dumps(value, indent=2) + "\n")
    print(json.dumps({"status": "built", "artifacts": value["artifacts"]}))


if __name__ == "__main__":
    main()
