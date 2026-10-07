"""Indexed visual composition, exact edit oracle, preview/export parity and independent lossless decode."""
import argparse
from fractions import Fraction
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("video_time_check", Path(__file__).with_name("video-time-check.py"))
timing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(timing)


def queries(data, reverse):
    """Independent linear scan over original ISO sample/edit tables, using Python Fraction."""
    scale = data["summary"]["mediaTimescale"]
    movie_scale = data["summary"]["movieTimescale"]
    end = timing.ratio(data["summary"]["movieEnd"])
    samples = sorted(data["samples"], key=lambda s: s["pts"])
    bounds = [(Fraction(s["pts"], scale), Fraction(samples[i+1]["pts"] if i+1 < len(samples)
               else s["pts"]+s["decodeDuration"], scale), s["pts"]) for i,s in enumerate(samples)]
    edits = [(Fraction(e["movieStart"], movie_scale), Fraction(e["movieStart"]+e["movieDuration"], movie_scale),
              None if e["mediaTime"] is None else Fraction(e["mediaTime"], scale), Fraction(e["rate1616"],65536)) for e in data["edits"]]
    if not edits: edits = [(Fraction(0), end, Fraction(0), Fraction(1))]
    count = -((-end.numerator * 24)//end.denominator)
    result = []
    for number in range(count):
        time = end - Fraction(number,24) if reverse else Fraction(number,24)
        side = "before" if reverse else "after"
        start, stop, media, rate = next(e for e in edits if timing.contains(e[0],e[1],time,side))
        pts = None
        if media is not None:
            at = media+(time-start)*rate
            pts = next((p for a,b,p in bounds if timing.contains(a,b,at,side if rate else "after")), None)
        result.append({"number":number,"time":{"numerator":str(time.numerator),"denominator":str(time.denominator)},"pts":pts})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", action="append", type=Path, required=True)
    parser.add_argument("--timing-evidence", type=Path, required=True)
    parser.add_argument("--worker-bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve(); output.mkdir(parents=True,exist_ok=False)
    bundle=args.worker_bundle.resolve(); pin=hashlib.sha256((bundle/"manifest.json").read_bytes()).hexdigest()
    evidence=args.timing_evidence.resolve(); recorded=json.loads((evidence/"report.json").read_text())
    assert recorded["status"]=="passed" and recorded["workerManifestSha256"]==pin
    assert len(args.input)==len(recorded["inputs"])
    ffmpeg=shutil.which("ffmpeg"); ffprobe=shutil.which("ffprobe")
    if not ffmpeg or not ffprobe: raise RuntimeError("independent FFmpeg tools required")
    def run(command,name,env=None):
        value=subprocess.run(list(map(str,command)),cwd=ROOT,env=env,capture_output=True,timeout=600)
        (output/(name+".stdout")).write_bytes(value.stdout);(output/(name+".stderr")).write_bytes(value.stderr)
        value.check_returncode();return value.stdout
    inputs=[]
    for i,path in enumerate(args.input):
        path=path.resolve(); entry=recorded["inputs"][i]
        assert hashlib.sha256(path.read_bytes()).hexdigest()==entry["sha256"]
        data=json.loads((evidence/f"{i}-timing.json").read_text())
        timing.verify_mapping(data)
        oracle={"trackId":data["summary"]["trackId"],"raw":data["frames"],"forward":queries(data,False),"reverse":queries(data,True)}
        oracle_path=output/f"{i}-oracle.json";oracle_path.write_text(json.dumps(oracle,indent=2)+"\n")
        env=dict(os.environ,MO_VISUAL_INPUT=str(path),MO_VISUAL_OUTPUT=str(output/str(i)),MO_VISUAL_ORACLE=str(oracle_path),
                 MO_MEDIA_TEST_BUNDLE=str(bundle),MO_MEDIA_TEST_BUNDLE_SHA256=pin)
        run(["cargo","test","--locked","--offline","-p","mo-video-runtime","--features","native","--test","native_visual","--","--ignored","--nocapture"],f"{i}-tests",env)
        actual=json.loads((output/str(i)/"report.json").read_text())
        assert len(actual["raw"])==entry["frames"]
        directions=[]
        for direction in actual["directions"]:
            name=direction["direction"];config=direction["config"];p=output/str(i)/f"{name}.mp4"
            decoded=output/str(i)/f"{name}-decoded.yuv"
            run([ffmpeg,"-v","error","-i",p,"-map","0:v:0","-pix_fmt","yuv420p","-fps_mode","passthrough","-f","rawvideo",decoded],f"{i}-{name}-decode")
            expected=(output/str(i)/f"{name}.yuv").read_bytes();assert decoded.read_bytes()==expected
            assert len(expected)==direction["frames"]*config["width"]*config["height"]*3//2
            streams=json.loads(run([ffprobe,"-v","error","-show_streams","-show_packets","-of","json",p],f"{i}-{name}-probe"))
            stream,=streams["streams"];assert stream["codec_name"]=="ffv1"
            base=Fraction(stream["time_base"]);packets=streams["packets"];assert len(packets)==direction["frames"]
            for n,packet in enumerate(packets): assert int(packet["pts"])*base==Fraction(n,24)
            peak=max(s["colorBytes"] for s in direction["stats"])
            assert peak<=config["width"]*config["height"]*16*3
            directions.append({"direction":name,"frames":direction["frames"],"gaps":sum(r["pts"] is None for r in oracle[name]),
                "peakNormalizedBytes":peak,"normalizedCacheHits":sum(s["colorCacheHit"] for s in direction["stats"]),
                "mp4Sha256":hashlib.sha256(p.read_bytes()).hexdigest(),"expectedAndDecodedSha256":hashlib.sha256(expected).hexdigest()})
        inputs.append({"input":path.name,"sha256":entry["sha256"],"rawFrames":entry["frames"],"directions":directions})
    report={"status":"passed","workerManifestSha256":pin,"inputs":inputs,
            "timingEvidenceSha256":hashlib.sha256((evidence/"report.json").read_bytes()).hexdigest(),
            "reference":run([ffmpeg,"-version"],"reference-version").decode().splitlines()[0],
            "scope":"Indexed source normalization, CPU composition and lossless native export with exact Python Fraction edit oracle. Explicit BT709 limited left-chroma input interpretation; these fixtures are opaque, square-pixel, identity-display videos. Actual pixels checked against timing evidence's independent decode; every scrub preview equals export input, independent FFV1 decode equals those bytes. Both source processes reaped on drop and cached cancellation. Not browser playback, product UI, GPU execution or full codec acceptance."}
    (output/"report.json").write_text(json.dumps(report,indent=2)+"\n");print(json.dumps(report))


if __name__=="__main__": main()
