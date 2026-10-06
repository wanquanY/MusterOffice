"""Actual worker raw decode + exact edit mapping, checked by independent decode and integer fractions."""
import argparse
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
from video_time_fixtures import variations

ROOT = Path(__file__).resolve().parents[2]


def ratio(value):
    return Fraction(int(value["numerator"]), int(value["denominator"]))


def contains(start, end, time, side):
    return start <= time < end if side == "after" else start < time <= end


def verify_mapping(data):
    scale = data["summary"]["mediaTimescale"]
    movie_scale = data["summary"]["movieTimescale"]
    frames = sorted(enumerate(data["samples"]), key=lambda row: row[1]["pts"])
    bounds = [(i, Fraction(s["pts"], scale), Fraction(frames[j+1][1]["pts"] if j+1 < len(frames)
              else s["pts"] + s["decodeDuration"], scale)) for j, (i, s) in enumerate(frames)]
    edits = [(i, Fraction(e["movieStart"], movie_scale), Fraction(e["movieStart"] + e["movieDuration"], movie_scale),
              None if e["mediaTime"] is None else Fraction(e["mediaTime"], scale), Fraction(e["rate1616"], 65536))
             for i, e in enumerate(data["edits"])]
    if not edits:
        duration = data["track"]["declaredTrackDuration"]
        edits = [(None, Fraction(0), Fraction(duration, movie_scale) if duration is not None else max(0, bounds[-1][2]), Fraction(0), Fraction(1))]
    for row in data["queries"]:
        time, side, result = ratio(row["time"]), row["side"], row["selection"]
        edit = next((e for e in edits if contains(e[1], e[2], time, side)), None)
        if edit is None:
            assert result["type"] == "outside", row
            continue
        identity, start, end, media, rate = edit
        assert result["edit"] == identity
        if media is None:
            expected = ("gap", start, end)
        else:
            at = media + (time-start)*rate
            media_side = side if rate else "after"
            frame = next((f for f in bounds if contains(f[1], f[2], at, media_side)), None)
            if frame is None:
                if not rate:
                    expected = ("gap", start, end)
                elif at < bounds[0][1] or (at == bounds[0][1] and media_side == "before"):
                    expected = ("gap", start, min(end, start+(bounds[0][1]-media)/rate))
                else:
                    expected = ("gap", max(start, start+(bounds[-1][2]-media)/rate), end)
            else:
                sample, first, last = frame
                expected = ("sample", max(start, start+(first-media)/rate) if rate else start,
                            min(end, start+(last-media)/rate) if rate else end)
                assert result["sample"] == sample and result["rawStart"] == first*scale and result["rawEnd"] == last*scale, row
                assert ratio(result["mediaTime"]) == at and result["mediaSide"] == media_side, row
                assert row["frameSha256"] == next(f["sha256"] for f in data["frames"] if f["pts"] == first*scale)
        assert (result["type"], ratio(result["movie"]["start"]), ratio(result["movie"]["end"])) == expected, row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, action="append", required=True)
    parser.add_argument("--worker-bundle", type=Path, required=True)
    parser.add_argument("--ffmpeg", type=Path)
    parser.add_argument("--variants", action="store_true", help="generate signed CTS and repeated/rate/dwell/empty edit cases from the first real input")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve(); output.mkdir(parents=True, exist_ok=False)
    bundle = args.worker_bundle.resolve()
    pin = hashlib.sha256((bundle/"manifest.json").read_bytes()).hexdigest()
    assert json.loads((bundle/"manifest.json").read_text())["profile"] == "musteroffice.native-media-worker/4"
    ffmpeg = args.ffmpeg or shutil.which("ffmpeg")
    if not ffmpeg: raise RuntimeError("independent ffmpeg required")
    ffmpeg = Path(ffmpeg).resolve(); ffprobe = ffmpeg.with_name("ffprobe")
    reports = []

    def run(command, name, env=None):
        result = subprocess.run(list(map(str, command)), cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output/(name+".stdout")).write_bytes(result.stdout); (output/(name+".stderr")).write_bytes(result.stderr)
        result.check_returncode()
        return result.stdout

    inputs = list(args.input)
    if args.variants:
        inputs.extend(variations(inputs[0].resolve(), ffmpeg, ffprobe, output, run))
    for i, path in enumerate(inputs):
        path = path.resolve()
        env = dict(os.environ, MO_MEDIA_TEST_BUNDLE=str(bundle), MO_MEDIA_TEST_BUNDLE_SHA256=pin,
                   MO_VIDEO_TIME_INPUT=str(path), MO_VIDEO_TIME_OUTPUT=str(output/f"{i}-timing.json"))
        run(["cargo", "test", "--offline", "--locked", "-p", "mo-native-media", "--test", "timing", "--", "--ignored", "--nocapture"], f"{i}-tests", env)
        data = json.loads((output/f"{i}-timing.json").read_text())
        stream = data["stream"]; scale = data["summary"]["mediaTimescale"]
        reference = json.loads(run([ffprobe, "-v", "error", "-ignore_editlist", "1", "-show_packets", "-show_streams", "-of", "json", path], f"{i}-packets"))
        actual_stream, = [s for s in reference["streams"] if s["index"] == stream["index"]]
        assert (int(actual_stream["id"], 0) & 0xffffffff) == data["summary"]["trackId"] == stream["containerTrackId"]
        packets = [p for p in reference["packets"] if p["stream_index"] == stream["index"]]
        assert len(packets) == len(data["samples"])
        # MOV demuxing subtracts the magnitude of the most negative CTS offset
        # from DTS, even with ignore_editlist=1 (mov_update_dts_shift). Its
        # reconstructed PTS remains in raw composition ticks. Derive this shift
        # independently from the indexed offsets; never rewrite the index.
        dts_shift = min(0x7fffffff, max(0, -min(s["pts"] - s["dts"] for s in data["samples"])))
        for actual, packet in zip(data["samples"], packets):
            assert actual["pts"] == int(packet["pts"])
            assert actual["dts"] - dts_shift == int(packet["dts"])
            assert actual["bytes"] == {"offset": int(packet["pos"]), "length": int(packet["size"])}
        framehash = output/f"{i}-reference.framehash"
        run([ffmpeg, "-v", "error", "-ignore_editlist", "1", "-copyts", "-i", path, "-map", f"0:{stream['index']}",
             "-pix_fmt", "yuv420p", "-fps_mode", "passthrough", "-enc_time_base", f"1:{scale}", "-avoid_negative_ts", "disabled",
             "-f", "framehash", "-hash", "sha256", framehash], f"{i}-frames")
        lines = framehash.read_text().splitlines()
        assert Fraction(next(s.split(":", 1)[1].strip() for s in lines if s.startswith("#tb 0:"))) == Fraction(1, scale)
        frames = [[v.strip() for v in line.split(",")] for line in lines if line and not line.startswith("#")]
        assert [(f["pts"], f["bytes"], f["sha256"]) for f in data["frames"]] == [(int(f[2]), int(f[4]), f[5]) for f in frames]
        verify_mapping(data)
        reports.append({"input": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "trackId": stream["containerTrackId"],
                        "frames": len(frames), "queries": len(data["queries"]), "seekChecks": data["seekChecks"],
                        "movieEnd": data["summary"]["movieEnd"], "rawSpan": data["summary"]["rawSpan"],
                        "referenceDtsShift": dts_shift})
    report = {"status": "passed", "workerManifestSha256": pin, "inputs": reports,
              "reference": run([ffmpeg, "-version"], "reference-version").decode().splitlines()[0],
              "scope": "Actual CPU decoded video pixels and raw PTS, indexed track identity, exact movie edit queries and seeks. Container timing is separate from codec admission. No audio edit mapping, Browser or product UI acceptance."}
    (output/"report.json").write_text(json.dumps(report, indent=2)+"\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
