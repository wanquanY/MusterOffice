"""Independently inspect/decode actual browser MP4 candidates; no mock codecs."""
import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import numpy as np


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    tool = shutil.which("ffmpeg")
    if not tool:
        raise ValueError("independent FFmpeg required")
    probe = str(Path(tool).with_name("ffprobe"))
    commands = []

    def run(command, name):
        commands.append(command)
        result = subprocess.run(command, capture_output=True, timeout=60)
        (output / (name + ".stdout")).write_bytes(result.stdout)
        (output / (name + ".stderr")).write_bytes(result.stderr)
        result.check_returncode()
        return result.stdout

    version = run([tool, "-version"], "tool-version").decode()
    report = json.loads((args.input / "report.json").read_text())
    checks = []
    for row in report["cases"]:
        name, info = row["name"], row["info"]
        path = str((args.input / (name + ".mp4")).resolve())
        value = json.loads(run([probe, "-v", "error", "-show_streams", "-show_packets", "-of", "json", path], name + "-probe"))
        streams, packets = value["streams"], value["packets"]
        video = streams[0]
        assert video["codec_name"] == "h264" and video["has_b_frames"] == 0
        assert Fraction(video["time_base"]) == Fraction(1, info["videoTimeScale"])
        assert int(video["start_pts"]) == 0 and int(video["duration_ts"]) == int(info["videoDuration"])
        assert video["width"] == info["width"] and video["height"] == info["height"]
        assert all(video.get(k) == v for k, v in {"color_primaries":"bt709", "color_transfer":"bt709", "color_space":"bt709", "color_range":"tv", "chroma_location":"left", "sample_aspect_ratio":"1:1", "field_order":"progressive"}.items())
        vp = [p for p in packets if p["stream_index"] == 0]
        period = info["videoTimeScale"] * info["frameRateDenominator"] // info["frameRateNumerator"]
        assert len(vp) == info["videoFrames"]
        assert [p["pts"] for p in vp] == [i * period for i in range(info["videoFrames"])]
        assert all(p["dts"] == p["pts"] for p in vp)
        assert vp[-1]["duration"] == int(info["videoDuration"]) - vp[-1]["pts"]
        raw = output / (name + ".i420")
        run([tool, "-v", "error", "-i", path, "-map", "0:v:0", "-fps_mode", "passthrough", "-pix_fmt", "yuv420p", "-f", "rawvideo", str(raw)], name + "-decode-video")
        assert raw.stat().st_size == info["width"] * info["height"] * 3 // 2 * info["videoFrames"]
        check = {"name":name, "sha256":hashlib.sha256(Path(path).read_bytes()).hexdigest(), "videoFrames":len(vp), "exactVideoDuration":str(info["videoDuration"])}
        decoded_video = np.fromfile(raw, dtype='uint8').reshape(info["videoFrames"], -1)
        original_video = np.empty_like(decoded_video)
        pixels = info["width"] * info["height"]
        for i in range(info["videoFrames"]):
            original_video[i, :pixels] = (16 + (np.arange(info["width"])[None, :] + np.arange(info["height"])[:, None] + i*5) % 220).flatten()
            original_video[i, pixels:pixels*5//4] = 100
            original_video[i, pixels*5//4:] = 150
        delta = decoded_video.astype('float64') - original_video
        check.update(videoInputRmsError=float(np.sqrt(np.mean(delta*delta))), videoInputMaxError=float(np.max(np.abs(delta))))
        samples, channels = int(info["audioSamples"]), info["audioChannels"]
        if samples:
            audio = streams[1]
            assert len(streams) == 2 and audio["codec_name"] == "aac" and audio["channels"] == channels
            assert Fraction(audio["time_base"]) == Fraction(1, info["audioSampleRate"])
            assert int(audio["start_pts"]) == 0 and int(audio["duration_ts"]) == samples
            pcm_path = output / (name + ".f32")
            run([tool, "-v", "error", "-i", path, "-map", "0:a:0", "-c:a", "pcm_f32le", "-f", "f32le", str(pcm_path)], name + "-decode-audio")
            pcm = np.fromfile(pcm_path, dtype='<f4').reshape(-1, channels)
            assert samples <= len(pcm) < samples + 1024
            at = np.arange(samples)
            original = np.stack([0.25 * np.sin(2 * np.pi * (937 if ch else 431) * at / info["audioSampleRate"]) * ((at >= 256) & (at % 8192 >= 128)) for ch in range(channels)], axis=1).astype('float32')
            # Search both directions. Use actual overlap so the very short tail
            # fixture participates too, and normalize by signal energy.
            shifts = range(-min(4096, samples // 2), min(4096, samples // 2) + 1)
            errors = []
            for shift in shifts:
                lo, hi = max(0, -shift), min(samples, len(pcm) - shift, 16384)
                a, b = original[lo:hi], pcm[lo + shift:hi + shift]
                errors.append(float(np.mean((a-b)**2) / max(float(np.mean(a*a)), 1e-9)))
            best = shifts[int(np.argmin(errors))]
            assert best == 0, (name, "decoded audio shifted", best)
            original_path = output / (name + "-original.f32")
            original.tofile(original_path)
            reference_path = output / (name + "-reference.m4a")
            reference_pcm = output / (name + "-reference.f32")
            run([tool, "-v", "error", "-f", "f32le", "-ar", str(info["audioSampleRate"]), "-ac", str(channels), "-i", str(original_path),
                 "-c:a", "aac", "-b:a", str(info["audioBitRate"]), "-movie_timescale", str(info["audioSampleRate"]), str(reference_path)], name + "-reference-encode")
            run([tool, "-v", "error", "-i", str(reference_path), "-c:a", "pcm_f32le", "-f", "f32le", str(reference_pcm)], name + "-reference-decode")
            reference = np.fromfile(reference_pcm, dtype='<f4').reshape(-1, channels)
            assert len(reference) >= samples
            ours_rms = float(np.sqrt(np.mean((pcm[:samples] - original)**2)))
            reference_rms = float(np.sqrt(np.mean((reference[:samples] - original)**2)))
            assert ours_rms <= reference_rms * 1.05 + 1e-5, (name, "AAC reference quality", ours_rms, reference_rms)
            check.update(audioSamples=samples, decodedAudioLagSamples=best, normalizedAudioError=errors[shifts.index(0)], referenceDecoderTailFrames=len(pcm)-samples)
            check.update(audioInputRmsError=ours_rms, independentAudioEncodeRmsError=reference_rms)
        else:
            assert len(streams) == 1
        checks.append(check)
    result = {"format":"musteroffice.browser-mux-independent-decode/1", "referenceTool":version, "commands":commands, "cases":checks,
              "scope":"Actual independent packet clocks, complete decode, audio phase. Reference decoder tail padding is outside the exact container presentation extent."}
    (output / "report.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(checks))


if __name__ == "__main__":
    main()
