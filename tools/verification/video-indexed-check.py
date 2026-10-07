"""Actual indexed source and compiled plans against independent pixels and exact edit oracle."""
import argparse
from fractions import Fraction
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import shutil

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("video_time_check", Path(__file__).with_name("video-time-check.py"))
timing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(timing)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", action="append", type=Path, required=True)
    parser.add_argument("--variants", action="store_true")
    parser.add_argument("--worker-bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve(); output.mkdir(parents=True, exist_ok=False)
    bundle = args.worker_bundle.resolve()
    pin = hashlib.sha256((bundle/"manifest.json").read_bytes()).hexdigest()

    def run(command, name, env=None):
        result = subprocess.run(list(map(str, command)), cwd=ROOT, env=env, capture_output=True, timeout=600)
        (output/(name+".stdout")).write_bytes(result.stdout); (output/(name+".stderr")).write_bytes(result.stderr)
        result.check_returncode()

    inputs = [p.resolve() for p in args.input]
    if args.variants:
        open_gop = output/"open-gop.mp4"
        ffmpeg = shutil.which("ffmpeg")
        if not ffmpeg: raise RuntimeError("independent FFmpeg required")
        run([ffmpeg,"-hide_banner","-loglevel","error","-f","lavfi","-i","testsrc2=size=128x96:rate=30:duration=4",
             "-c:v","libx264","-x264-params","keyint=30:min-keyint=30:scenecut=0:open-gop=1:bframes=3","-pix_fmt","yuv420p","-n",open_gop],"open-gop-fixture")
        inputs.append(open_gop)
    command = ["python3", "tools/verification/video-time-check.py", "--worker-bundle", bundle, "--output", output/"timing"]
    for path in inputs: command.extend(["--input", path])
    if args.variants: command.append("--variants")
    run(command, "independent-time")
    evidence = json.loads((output/"timing/report.json").read_text())
    assert evidence["status"] == "passed" and evidence["workerManifestSha256"] == pin
    if args.variants: inputs.extend([output/"timing/signed-timing.mp4", output/"timing/edits-timing.mp4"])
    reports = []
    for i, path in enumerate(inputs):
        assert hashlib.sha256(path.read_bytes()).hexdigest() == evidence["inputs"][i]["sha256"]
        time_path = output/f"timing/{i}-timing.json"
        env = dict(os.environ, MO_INDEXED_INPUT=str(path), MO_INDEXED_TIMING=str(time_path), MO_INDEXED_OUTPUT=str(output/f"{i}-source.json"),
                   MO_MEDIA_TEST_BUNDLE=str(bundle), MO_MEDIA_TEST_BUNDLE_SHA256=pin)
        run(["cargo", "test", "--offline", "--locked", "-p", "mo-video-source", "--features", "native", "--test", "native_indexed", "--", "--ignored", "--nocapture"], f"{i}-source-tests", env)
        result = json.loads((output/f"{i}-source.json").read_text())
        raw = json.loads(time_path.read_text())
        rows = result["scrub"]+result["plans"]
        raw["queries"] = rows
        timing.verify_mapping(raw)
        peak = 0
        for row in rows:
            selected, picture, stats = row["selection"], row["picture"], row["stats"]
            assert (picture is not None) == (selected["type"] == "sample")
            if picture:
                assert picture["pts"] == selected["rawStart"]
                expected, = [f for f in raw["frames"] if f["pts"] == selected["rawStart"]]
                assert (picture["sha256"], picture["bytes"]) == (expected["sha256"], expected["bytes"])
                assert Fraction(picture["timeBase"]["numerator"], picture["timeBase"]["denominator"]) == Fraction(1,raw["summary"]["mediaTimescale"])
            else: assert stats["decodedFrames"] == stats["seeks"] == 0
            peak = max(peak, stats["retainedBytes"])
            assert stats["decodedFrames"] <= 1000
        end = timing.ratio(raw["summary"]["movieEnd"])
        for row in result["plans"]:
            at = row["frameNumber"]*Fraction(1001,30000)
            assert timing.ratio(row["time"]) == (end-at if row["reverse"] else at)
            assert row["side"] == ("before" if row["reverse"] else "after")
        assert peak <= max(f["bytes"] for f in raw["frames"])*4
        assert any(r["stats"]["cacheHit"] for r in rows) and any(r["stats"]["seeks"] for r in rows)
        reports.append({"input":path.name,"sha256":evidence["inputs"][i]["sha256"],"scrubQueries":len(result["scrub"]),
            "compiledPlanQueries":len(result["plans"]),"peakRetainedBytes":peak,"cacheHits":sum(r["stats"]["cacheHit"] for r in rows),
            "seeks":sum(r["stats"]["seeks"] for r in rows)})
    report = {"status":"passed","workerManifestSha256":pin,"inputs":reports,
        "timingEvidenceSha256":hashlib.sha256((output/"timing/report.json").read_bytes()).hexdigest(),
        "scope":"Actual raw worker frames selected through retained exact container maps for compiled forward/reverse plans and boundary scrubbing. Bounded active frame cache and two-worker cancellation/reap checked. Not Browser, product UI, audio edits or full acceptance."}
    (output/"report.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
