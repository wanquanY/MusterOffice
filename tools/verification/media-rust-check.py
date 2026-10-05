"""Exercise the safe Rust adapter against newly generated native media evidence."""
import argparse
from array import array
from fractions import Fraction
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--component", type=Path, required=True)
    parser.add_argument("--native-evidence", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    evidence = args.native_evidence.resolve()
    native = json.loads((evidence / "report.json").read_text())
    component = args.component.resolve()
    record = json.loads((component / "native-build.json").read_text())
    assert native["status"] == "passed" and native["component"] == record, "native evidence/component mismatch"
    fixture = evidence / "bframes-vfr.mp4"
    assert sha(fixture) == native["fixtureSha256"], "fixture changed after independent check"
    env = dict(os.environ)
    env["MO_FFMPEG_LIB_DIR"] = str(component)
    env["MO_MEDIA_TEST_FIXTURE"] = str(fixture)
    env["MO_MEDIA_TEST_OUTPUT"] = str(output / "rust")
    commands = []

    def run(command, name):
        commands.append(list(map(str, command)))
        result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output / (name + ".stdout")).write_bytes(result.stdout)
        (output / (name + ".stderr")).write_bytes(result.stderr)
        result.check_returncode()

    run(["cargo", "test", "--offline", "--locked", "-p", "mo-media-ffmpeg"], "ownership-tests")
    run(["cargo", "test", "--offline", "--locked", "-p", "mo-media-ffmpeg", "--test", "real_media", "--", "--ignored", "--nocapture"], "real-media")
    # This is the same external reference tool used by the C gate. It is not a
    # linkage/distribution dependency of either the Rust or native component.
    reference_tool = native["commands"][0][0]
    raw = output / "reference.yuv"
    run([reference_tool, "-hide_banner", "-loglevel", "error", "-i", str(fixture), "-map", "0:v:0", "-fps_mode", "passthrough", "-pix_fmt", "yuv420p", "-f", "rawvideo", str(raw)], "reference-video")
    assert sha(raw) == sha(output / "rust/video.yuv"), "Rust active pixel bytes differ from independent decoder"
    rows = json.loads((output / "rust/frames.json").read_text())
    lines = (evidence / "video.framemd5").read_text().splitlines()
    timebase = Fraction(next(s for s in lines if s.startswith("#tb 0:")).split(":", 1)[1].strip())
    expected_pts = [int(s.split(",")[2]) * timebase for s in lines if s and not s.startswith("#")]
    pixels = raw.read_bytes()
    frame_bytes = 128 * 96 * 3 // 2
    expected_hashes = [hashlib.sha256(pixels[i:i+frame_bytes]).hexdigest() for i in range(0, len(pixels), frame_bytes)]
    expected = list(zip(expected_pts, expected_hashes))
    assert len(expected_pts) == len(expected_hashes) == 77
    for phase, generation in [("linear", 1), ("forward", 2), ("backward", 3)]:
        frames = rows[phase]
        assert frames and all(f["generation"] == generation for f in frames)
        assert all(f["colors"] == [1, 1, 1, 1] and f["format"] == "yuv420p" for f in frames)
        actual = [(int(f["pts"]) * Fraction(*f["timeBase"]), f["sha256"]) for f in frames]
        if phase == "forward":
            assert actual == expected[-len(actual):] and actual[0][0] <= 1 <= actual[-1][0]
        else:
            assert actual == expected, "Rust frame timestamps/content differ"
    ours = array('f', (output / "rust/audio.f32").read_bytes())
    reference = array('f', (evidence / "reference.f32").read_bytes())
    assert len(ours) == len(reference) == 144000
    cursor = 0
    for frame in rows["audio"]:
        assert frame["format"] == "fltp" and frame["generation"] == 1
        assert frame["pts"] * Fraction(*frame["timeBase"]) == Fraction(cursor, 48000)
        cursor += frame["samples"]
    assert cursor == 144000 and all(math.isfinite(v) for v in ours)
    max_error = max(abs(a-b) for a,b in zip(ours, reference))
    rms_error = math.sqrt(sum((a-b)**2 for a,b in zip(ours, reference)) / len(ours))
    assert max_error < 1e-5 and rms_error < 1e-6
    report = {"format": "musteroffice.media-rust-check/1", "status": "passed", "commands": commands,
              "nativeEvidenceSha256": sha(evidence / "report.json"), "componentRecordSha256": sha(component / "native-build.json"),
              "fixtureSha256": sha(fixture), "videoFrames": 77, "videoSha256": sha(raw), "audioSamples": cursor,
              "audioMaxAbsoluteError": max_error, "audioRmsError": rms_error,
              "scope": "Actual safe Rust callback/ownership and decode bytes/timestamps/seek; no worker isolation, browser, GPU or product acceptance."}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ["status", "videoFrames", "audioSamples", "fixtureSha256"]}))


if __name__ == "__main__":
    main()
