"""Encode actual typed Rust inputs and independently inspect the resulting MP4 candidates."""
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
    parser.add_argument("--hardware", action="store_true", help="require actual VideoToolbox H.264; failure is not skipped")
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--project", action="store_true", help="export compiled project composition and audio mix")
    parser.add_argument("--worker-bundle", type=Path, help="encode/decode project files through a verified disposable worker")
    args = parser.parse_args()
    if args.worker_bundle and not args.project:
        parser.error("--worker-bundle requires --project")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    component = args.component.resolve()
    record = json.loads((component / "native-build.json").read_text())
    env = dict(os.environ)
    env["MO_FFMPEG_LIB_DIR"] = str(component)
    output_key = "MO_PROJECT_EXPORT_OUTPUT" if args.project else "MO_ENCODE_TEST_OUTPUT"
    hardware_key = "MO_PROJECT_EXPORT_HARDWARE" if args.project else "MO_ENCODE_TEST_HARDWARE"
    env[output_key] = str(output / "exports")
    env.pop(hardware_key, None)
    if args.hardware:
        env[hardware_key] = "1"
    if args.worker_bundle:
        bundle = args.worker_bundle.resolve()
        manifest = json.loads((bundle / "manifest.json").read_text())
        assert manifest["profile"] == "musteroffice.native-media-worker/5"
        assert manifest["componentRecordSha256"] == sha(component / "native-build.json")
        env["MO_MEDIA_TEST_BUNDLE"] = str(bundle)
        env["MO_MEDIA_TEST_BUNDLE_SHA256"] = sha(bundle / "manifest.json")
    commands = []

    def run(command, name):
        commands.append(list(map(str, command)))
        result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output / (name + ".stdout")).write_bytes(result.stdout)
        (output / (name + ".stderr")).write_bytes(result.stderr)
        result.check_returncode()
        return result.stdout

    tool = shutil.which(args.ffmpeg)
    if not tool:
        raise ValueError("independent reference tool unavailable")
    probe = str(Path(tool).with_name("ffprobe"))
    version = run([tool, "-version"], "reference-version").decode()
    # Cargo's build gate checks current wrapper/patch/header/library/alias identity
    # before linking. Both default error paths and actual finite exports run here.
    suite = "project_export" if args.project else "encoding"
    evidence = "actual_project_export_files" if args.project else "real_export_profiles"
    package = "mo-native-media" if args.worker_bundle else "mo-media-ffmpeg"
    behavior = ["project_composition", "--", "--ignored"] if args.worker_bundle else []
    run(["cargo", "test", "--offline", "--locked", "-p", package, "--test", suite, *behavior], "behavior")
    if args.worker_bundle:
        run(["cargo", "test", "--offline", "--locked", "-p", package, "--test", "encoding", "--", "--ignored"], "worker-failures")
    run(["cargo", "test", "--offline", "--locked", "-p", package, "--test", suite, evidence, "--", "--ignored", "--nocapture"], "exports")
    exports = output / "exports"
    cases = json.loads((exports / "exports.json").read_text())
    assert len(cases) == (3 if args.project else 6) * (2 if args.hardware else 1)
    for row in cases:
        name, c = row["name"], row["config"]
        path = exports / (name + ".mp4")
        info = json.loads(run([probe, "-v", "error", "-show_streams", "-show_packets", "-of", "json", str(path)], name + "-probe"))
        streams, packets = info["streams"], info["packets"]
        video = streams[0]
        tb = Fraction(video["time_base"])
        assert tb == Fraction(1, c["videoTimeScale"])
        assert int(video["start_pts"]) == 0 and int(video["duration_ts"]) == c["videoDuration"]
        assert video["width"] == c["width"] and video["height"] == c["height"]
        assert all(video.get(k) == v for k, v in {"color_primaries": "bt709", "color_transfer": "bt709", "color_space": "bt709", "color_range": "tv", "sample_aspect_ratio": "1:1"}.items())
        vp = [p for p in packets if p["stream_index"] == 0]
        period = c["videoTimeScale"] * c["frameRateDen"] // c["frameRateNum"]
        assert len(vp) == c["videoFrames"]
        assert sorted(p["pts"] for p in vp) == list(range(0, period*c["videoFrames"], period))
        assert all(p["pts"] >= p["dts"] for p in vp)
        for index in range(len(streams)):
            pp = [p for p in packets if p["stream_index"] == index]
            assert all(a["dts"] < b["dts"] for a, b in zip(pp, pp[1:]))
        dts = [p["dts"] * Fraction(streams[p["stream_index"]]["time_base"]) for p in packets]
        assert dts == sorted(dts), "global decode-time interleave"
        reference_video = output / (name + "-reference.yuv")
        run([tool, "-v", "error", "-i", str(path), "-map", "0:v:0", "-fps_mode", "passthrough", "-pix_fmt", "yuv420p", "-f", "rawvideo", str(reference_video)], name + "-decode-video")
        assert reference_video.read_bytes() == (exports / (name + ".decoded.yuv")).read_bytes()
        actual = reference_video.read_bytes()
        expected = (exports / (name + ".input.yuv")).read_bytes()
        assert len(actual) == len(expected) == c["width"]*c["height"]*3//2*c["videoFrames"]
        mse = sum((a-b)**2 for a, b in zip(actual, expected))/len(actual)
        row["videoPsnrDb"] = 10*math.log10(255**2/mse) if mse else None
        lossless = name.startswith("lossless")
        if lossless:
            assert actual == expected
        else:
            assert video["codec_name"] == "h264" and video["chroma_location"] == "left"
            assert row["videoPsnrDb"] is None or row["videoPsnrDb"] > 35
            if c["videoFrames"] > 16:
                assert any(a["pts"] > b["pts"] for a, b in zip(vp, vp[1:])), "fixture must exercise B-frame reordering"
        samples = c["audioSamples"]
        row["referenceAudioTailFrames"] = 0
        if samples:
            audio = streams[1]
            assert Fraction(audio["time_base"]) == Fraction(1, c["audioSampleRate"])
            assert int(audio["start_pts"]) == 0 and int(audio["duration_ts"]) == samples
            assert audio["channels"] == 2
            reference_pcm = output / (name + "-reference.f32")
            run([tool, "-v", "error", "-i", str(path), "-map", "0:a:0", "-c:a", "pcm_f32le", "-f", "f32le", str(reference_pcm)], name + "-decode-audio")
            reference = array('f', reference_pcm.read_bytes())
            ours = array('f', (exports / (name + ".decoded.f32")).read_bytes())
            original = array('f', (exports / (name + ".input.f32")).read_bytes())
            assert len(ours) == len(original) == samples*2
            # Some older CLI demuxers retain AAC's final codec padding. The
            # native result and stream extent above must be exact without trim.
            extra = len(reference)-len(ours)
            assert extra == 0 if lossless else (0 <= extra < 2048 and extra % 2 == 0)
            row["referenceAudioTailFrames"] = extra//2
            error = max(abs(a-b) for a, b in zip(reference, ours))
            assert error < 1e-5
            row["independentAudioMaxError"] = error
            if lossless:
                assert ours == original
            else:
                rms = math.sqrt(sum((a-b)**2 for a, b in zip(ours, original))/len(ours))
                # Lossy AAC's short-clip startup transient is codec-dependent.
                # Compare the same input/rate/bitrate with an independent encoder
                # instead of inventing a universal perceptual-quality threshold.
                reference_aac = output / (name + "-reference.m4a")
                reference_aac_pcm = output / (name + "-reference-encoded.f32")
                run([tool, "-v", "error", "-f", "f32le", "-ar", str(c["audioSampleRate"]), "-ac", "2", "-i", str(exports / (name + ".input.f32")), "-c:a", "aac", "-b:a", str(c["audioBitRate"]), "-movie_timescale", str(c["audioSampleRate"]), str(reference_aac)], name + "-reference-encode")
                run([tool, "-v", "error", "-i", str(reference_aac), "-c:a", "pcm_f32le", "-f", "f32le", str(reference_aac_pcm)], name + "-reference-redecode")
                reference_encoded = array('f', reference_aac_pcm.read_bytes())
                assert len(reference_encoded) >= len(original)
                reference_rms = math.sqrt(sum((a-b)**2 for a, b in zip(reference_encoded, original))/len(original))
                assert rms <= reference_rms*1.05 + 1e-5
                row["audioInputRmsError"] = rms
                row["independentEncodeRmsError"] = reference_rms
        else:
            assert len(streams) == 1
        row["outputSha256"] = sha(path)
    report = {"format": "musteroffice.project-export-check/1" if args.project else "musteroffice.native-encoding-check/1", "status": "passed",
              "platform": platform.platform(), "machine": platform.machine(), "hardwareRequired": args.hardware,
              "component": record, "workerManifestSha256": env["MO_MEDIA_TEST_BUNDLE_SHA256"] if args.worker_bundle else None, "referenceTool": version, "commands": commands, "cases": cases,
              "scope": ("Compiled project with graphic clips, alpha, gap matte, resampled gain-adjusted mono-to-stereo audio, fractional clocks and real native MP4. Worker isolation is exercised only when workerManifestSha256 is present. Not decoded visual media, effects/text, browser or product acceptance." if args.project else "Typed native encode/mux/candidate sink, precise presentation/audio extent, B-frame drain, actual independent decode, lossless audit and failure paths. Not project-render export, worker isolation, browser/device playback, delivery bitrate quality, performance or product acceptance.")}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"status": "passed", "cases": len(cases), "hardware": args.hardware}))


if __name__ == "__main__":
    main()
