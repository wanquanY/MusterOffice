"""Real local media checks for the fixed callback ABI. No product/renderer acceptance claim."""
import argparse
from array import array
from fractions import Fraction
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--component", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--ffmpeg", default="ffmpeg", help="fixture/reference tool only, never loaded by the component")
    parser.add_argument("--sanitize", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    component = args.component.resolve()
    record = json.loads((component / "native-build.json").read_text())
    lock = json.loads((ROOT / "components/ffmpeg/lock.json").read_text())
    if record["lock"] != lock or record["upstream"]["lock"] != lock:
        raise ValueError("component source lock mismatch")
    if record["sanitizers"] != args.sanitize:
        raise ValueError("bridge sanitizer profile mismatch")
    expected_sources = {"mo_media.c", "mo_media_decode.c", "mo_media.h", "mo_media_internal.h"}
    if {row["path"] for row in record["sources"]} != expected_sources:
        raise ValueError("component source inventory mismatch")
    for row in record["sources"]:
        if sha(ROOT / "components/ffmpeg" / row["path"]) != row["sha256"]:
            raise ValueError("component is stale: rebuild the bridge from current sources")
    for row in record["artifacts"] + record["upstream"]["artifacts"] + record["upstream"]["headers"]:
        path = component / row["path"]
        if sha(path) != row["sha256"] or path.stat().st_size != row["byteLength"]:
            raise ValueError("component digest mismatch")
    commands = []

    def run(command, name):
        commands.append(list(map(str, command)))
        env = dict(os.environ)
        env["UBSAN_OPTIONS"] = "halt_on_error=1:print_stacktrace=1"
        env["ASAN_OPTIONS"] = "halt_on_error=1"  # leak detection follows platform support
        result = subprocess.run(command, capture_output=True, text=True, timeout=120, env=env)
        (output / (name + ".stdout")).write_text(result.stdout)
        (output / (name + ".stderr")).write_text(result.stderr)
        result.check_returncode()
        return result.stdout

    tool = shutil.which(args.ffmpeg)
    if tool is None:
        raise ValueError("fixture FFmpeg tool unavailable")
    version = run([tool, "-hide_banner", "-version"], "fixture-tool")
    probe_tool = str(Path(tool).with_name("ffprobe"))
    lib = component / "install/lib"
    probe = output / "media-probe"
    flags = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-g"] if args.sanitize else []
    run(["clang", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", *flags,
         "-I" + str(ROOT / "components/ffmpeg"), "-I" + str(component / "install/include"),
         str(ROOT / "tools/verification/media-probe.c"), "-L" + str(lib), "-Wl,-rpath," + str(lib), "-lmo_media", "-lavutil", "-o", str(probe)], "probe-build")
    fixture = output / "bframes-vfr.mp4"
    # VFR gaps, fractional frame rate, two B frames, predictable keyframe cadence, known SDR tags.
    run([tool, "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=128x96:rate=30000/1001:duration=3",
         "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=3", "-map", "0:v", "-map", "1:a",
         "-vf", "select='not(eq(mod(n,7),3))',setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=tv", "-fps_mode", "vfr", "-c:v", "libx264", "-preset", "medium",
         "-g", "30", "-bf", "2", "-pix_fmt", "yuv420p", "-color_primaries", "bt709", "-color_trc", "bt709",
         "-colorspace", "bt709", "-color_range", "tv", "-c:a", "aac", "-movflags", "+faststart", str(fixture)], "fixture-generate")
    fixture_info = json.loads(run([probe_tool, "-v", "error", "-select_streams", "v:0", "-show_packets", "-show_streams", "-show_entries", "packet=pts,dts:stream=color_primaries,color_transfer,color_space,color_range", "-of", "json", str(fixture)], "fixture-packets"))
    assert all(fixture_info["streams"][0].get(k) == v for k,v in {"color_primaries":"bt709", "color_transfer":"bt709", "color_space":"bt709", "color_range":"tv"}.items()), "fixture lacks declared color tags"
    packets = fixture_info["packets"]
    assert any(p.get("pts") != p.get("dts") for p in packets), "fixture lacks decode/presentation reordering"
    reference = output / "video.framemd5"
    run([tool, "-hide_banner", "-loglevel", "error", "-i", str(fixture), "-map", "0:v:0", "-fps_mode", "passthrough", "-f", "framemd5", str(reference)], "reference-video")
    lines = reference.read_text().splitlines()
    timebase = Fraction(next(s for s in lines if s.startswith("#tb 0:")).split(":", 1)[1].strip())
    expected = []
    for line in lines:
        if not line or line.startswith("#"):
            continue
        fields = [v.strip() for v in line.split(",")]
        expected.append((int(fields[2]) * timebase, int(fields[4]), fields[5]))
    decoded = [json.loads(s) for s in run([str(probe), str(fixture), "video"], "decode-video").splitlines()]
    assert decoded[-1]["status"] == "passed"
    linear = []
    for phase, generation in [("linear", 1), ("seek-forward", 2), ("seek-backward", 3), ("short-reads", 1)]:
        values = [r for r in decoded if r.get("phase") == phase]
        assert values and all(r["generation"] == generation for r in values)
        actual = [(int(r["pts"]) * Fraction(*r["timeBase"]), r["bytes"], r["md5"]) for r in values]
        assert actual == sorted(actual), "frames are not in presentation order"
        if phase == "seek-forward":
            assert actual == expected[-len(actual):], "seek changed decoded content or timestamp"
            assert actual[0][0] <= 1 <= actual[-1][0], "seek did not return the required preroll"
        else:
            assert actual == expected, "decoded frames differ from the independent tool"
        if phase == "linear":
            linear = actual
    assert len({linear[i+1][0]-linear[i][0] for i in range(len(linear)-1)}) > 1, "fixture is not VFR"
    native_pcm = output / "native.f32"
    audio = [json.loads(s) for s in run([str(probe), str(fixture), "audio", str(native_pcm)], "decode-audio").splitlines()]
    assert audio[-1]["status"] == "passed"
    pcm = output / "reference.f32"
    # This known source contains exactly 144000 samples. Older reference CLI releases can leave
    # AAC tail padding; explicitly trim that reference, never trim our result to make it pass.
    run([tool, "-hide_banner", "-loglevel", "error", "-i", str(fixture), "-map", "0:a:0", "-af", "atrim=end_sample=144000", "-c:a", "pcm_f32le", "-f", "f32le", str(pcm)], "reference-audio")
    audio_linear = [r for r in audio if r.get("phase") == "linear"]
    assert all(r["format"] == "fltp" for r in audio_linear)
    sample_count = sum(r["samples"] for r in audio_linear)
    assert sample_count == 144000 and native_pcm.stat().st_size == pcm.stat().st_size == sample_count * 4, "audio priming/padding sample count differs"
    next_sample = 0
    for frame in audio_linear:
        assert int(frame["pts"]) * Fraction(*frame["timeBase"]) == Fraction(next_sample, 48000), "audio timestamp discontinuity"
        next_sample += frame["samples"]
    ours = array('f', native_pcm.read_bytes())
    reference_pcm = array('f', pcm.read_bytes())
    assert all(math.isfinite(v) for v in ours), "non-finite decoded sample"
    max_error = max(abs(a-b) for a,b in zip(ours,reference_pcm))
    rms_error = math.sqrt(sum((a-b)**2 for a,b in zip(ours,reference_pcm))/sample_count)
    assert max_error < 1e-5 and rms_error < 1e-6, "AAC decoded PCM differs from reference"
    assert [(r["pts"], r["samples"], r["md5"]) for r in audio_linear] == [(r["pts"], r["samples"], r["md5"]) for r in audio if r.get("phase") == "short-reads"]
    report = {"format": "musteroffice.media-callback-check/1", "status": "passed", "platform": platform.platform(), "machine": platform.machine(),
              "component": record, "fixtureTool": version, "commands": commands, "videoFrames": len(linear), "audioSamples": sample_count,
              "fixtureSha256": sha(fixture), "videoReferenceSha256": sha(reference), "audioReferenceSha256": sha(pcm),
              "audioMaxAbsoluteError": max_error, "audioRmsError": rms_error,
              "scope": "Synthetic real H.264 B-frame/VFR and AAC decoding, drain, seek generations, plane bytes, callback I/O, budgets and cancellation. No renderer, Rust host, worker isolation, hardware, cross-platform or product acceptance."}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: report[k] for k in ["status", "videoFrames", "audioSamples", "fixtureSha256"]}))


if __name__ == "__main__":
    main()
