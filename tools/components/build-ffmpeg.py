"""Build fixed LGPL media libraries and our callback-only ABI; never use system FFmpeg.

Libraries and bridge are separate stages so changing our ABI does not rebuild the codec source.
Inputs are verified; upstream sources and all binary artifacts remain in the ignored directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
from native_platform import native_platform

ROOT = Path(__file__).resolve().parents[2]
COMPONENT = ROOT / "components/ffmpeg"
LOCK = json.loads((COMPONENT / "lock.json").read_text())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifacts(directory, names):
    return [{"path": name, "sha256": sha(directory / name), "byteLength": (directory / name).stat().st_size} for name in names]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", choices=["libraries", "bridge"], required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--jobs", type=int, default=min(8, os.cpu_count() or 1))
    parser.add_argument("--sanitize", action="store_true", help="instrument C code with address/undefined sanitizers; must match both stages")
    args = parser.parse_args()
    if not 1 <= args.jobs <= 32:
        parser.error("jobs must be in 1..=32")
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    target = native_platform()
    commands = []

    def run(command, cwd=directory):
        commands.append(list(map(str, command)))
        with (directory / (args.stage + "-build.log")).open("a") as log:
            subprocess.run(command, cwd=cwd, check=True, stdout=log, stderr=log)

    compiler = subprocess.check_output(["clang", "--version"], text=True)
    if args.stage == "libraries":
        if args.archive is None:
            parser.error("libraries stage requires --archive; downloads are explicit")
        archive = args.archive.resolve()
        if archive.stat().st_size != LOCK["byteLength"] or sha(archive) != LOCK["sha256"]:
            raise ValueError("FFmpeg source digest mismatch")
        source = directory / "source"
        source.mkdir(exist_ok=False)
        with tarfile.open(archive) as tar:
            tar.extractall(source, filter="data")
        upstream = source / ("ffmpeg-" + LOCK["version"])
        prefix = directory / "install"
        flags = ["--prefix=" + str(prefix), "--cc=clang", "--disable-autodetect", "--disable-everything",
                 "--disable-network", "--disable-protocols", "--disable-programs", "--disable-doc", "--disable-debug",
                 "--disable-gpl", "--disable-nonfree", "--disable-version3", "--disable-static", "--enable-shared",
                 "--disable-avdevice", "--disable-avfilter", "--enable-avformat", "--enable-avcodec", "--enable-avutil",
                 "--enable-swscale", "--enable-swresample", "--enable-pthreads", "--enable-runtime-cpudetect",
                 "--enable-demuxer=mov,matroska,wav,ogg,flac,mp3,aac,avi",
                 "--enable-muxer=mp4,mov,matroska,webm,wav,flac,ogg",
                 "--enable-parser=h264,hevc,vp8,vp9,aac,aac_latm,mpegaudio,flac,opus,vorbis",
                 "--enable-decoder=h264,hevc,vp8,vp9,prores,ffv1,rawvideo,mjpeg,aac,opus,vorbis,flac,mp3,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le",
                 "--enable-encoder=aac,flac,ffv1,pcm_s16le,pcm_f32le,rawvideo",
                 "--extra-cflags=-O2 -ffp-contract=off" + (" -fsanitize=address,undefined -fno-omit-frame-pointer -g" if args.sanitize else "")]
        if args.sanitize:
            flags += ["--extra-ldflags=-fsanitize=address,undefined"]
        if target["os"] == "macos":
            flags += ["--install-name-dir=@rpath", "--enable-videotoolbox", "--enable-audiotoolbox", "--enable-encoder=h264_videotoolbox,hevc_videotoolbox"]
        run([str(upstream / "configure"), *flags], upstream)
        config = (upstream / "config.h").read_text()
        for option in ["GPL", "GPLV3", "NONFREE", "VERSION3", "NETWORK"]:
            if f"#define CONFIG_{option} 0" not in config:
                raise ValueError("unexpected component configuration: " + option)
        run(["make", "-j" + str(args.jobs)], upstream)
        run(["make", "install"], upstream)
        lib = prefix / "lib"
        names = sorted(str(p.relative_to(directory)) for p in lib.iterdir() if p.is_file() and not p.is_symlink() and (p.suffix == ".dylib" or ".so." in p.name))
        if len(names) != len(LOCK["libraries"]):
            raise ValueError("unexpected shared library closure")
        for name in ["COPYING.LGPLv2.1", "LICENSE.md"]:
            shutil.copyfile(upstream / name, directory / name)
        (directory / "configure.txt").write_text(json.dumps(flags, indent=2) + "\n")
        headers = sorted(str(p.relative_to(directory)) for p in (prefix / "include").rglob("*.h"))
        record = {"format": "musteroffice.ffmpeg-libraries/1", "lock": LOCK, "nativePlatform": target, "sanitizers": args.sanitize,
                  "compiler": compiler, "commands": commands, "artifacts": artifacts(directory, names), "headers": artifacts(directory, headers)}
        (directory / "ffmpeg-build.json").write_text(json.dumps(record, indent=2) + "\n")
    else:
        record = json.loads((directory / "ffmpeg-build.json").read_text())
        if record["lock"] != LOCK or record["nativePlatform"] != target or record["sanitizers"] != args.sanitize:
            raise ValueError("component source/profile/target mismatch")
        for artifact in record["artifacts"] + record["headers"]:
            if sha(directory / artifact["path"]) != artifact["sha256"]:
                raise ValueError("shared library digest mismatch")
        include = directory / "install/include"
        lib = directory / "install/lib"
        suffix = ".dylib" if target["os"] == "macos" else ".so"
        bridge = lib / ("libmo_media" + suffix)
        inputs = [COMPONENT / name for name in ["mo_media.c", "mo_media_decode.c"]]
        flags = ["-std=c11", "-O2", "-ffp-contract=off", "-fPIC", "-fvisibility=hidden", "-Wall", "-Wextra", "-Werror", "-I" + str(include), "-I" + str(COMPONENT)]
        if args.sanitize:
            flags += ["-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-g"]
        # Linux DT_RPATH also resolves FFmpeg's transitive sibling libraries after relocation.
        linker = ["-dynamiclib", "-Wl,-install_name,@rpath/" + bridge.name, "-Wl,-rpath,@loader_path"] if target["os"] == "macos" else ["-shared", "-Wl,--disable-new-dtags,-rpath,$ORIGIN", "-Wl,-z,origin"]
        run(["clang", *flags, *map(str, inputs), "-L" + str(lib), *linker, "-lavformat", "-lavcodec", "-lavutil", "-lswscale", "-lswresample", "-o", str(bridge)])
        source_names = ["mo_media.c", "mo_media_decode.c", "mo_media.h", "mo_media_internal.h"]
        record = {"format": "musteroffice.ffmpeg-bridge/1", "lock": LOCK, "nativePlatform": target, "sanitizers": args.sanitize,
                  "compiler": compiler, "commands": commands, "upstream": record,
                  "sources": artifacts(COMPONENT, source_names), "artifacts": artifacts(directory, [str(bridge.relative_to(directory))])}
        (directory / "native-build.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps({"stage": args.stage, "artifacts": record["artifacts"]}))


if __name__ == "__main__":
    main()
