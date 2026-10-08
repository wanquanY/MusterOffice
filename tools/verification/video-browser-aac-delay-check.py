"""Measure actual AudioEncoder/AudioDecoder waveform delay; never generate a fixup constant."""
import argparse
import hashlib
import json
from pathlib import Path
import numpy as np


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    raw = args.probe.read_bytes()
    probe = json.loads(raw)
    results = []
    for case in probe["cases"]:
        info = case["info"]
        channels, samples = info["audioChannels"], int(info["audioSamples"])
        if not channels:
            continue
        decoded = np.concatenate([np.array(block["data"], dtype='float32').reshape(-1, channels) for block in case["decoded"]])
        at = np.arange(samples)
        original = np.stack([0.25 * np.sin(2 * np.pi * (937 if ch else 431) * at / info["audioSampleRate"]) * ((at >= 256) & (at % 8192 >= 128)) for ch in range(channels)], axis=1).astype('float32')
        errors = []
        for shift in range(4097):
            hi = min(samples, len(decoded) - shift, 16384)
            if hi < min(samples, 1024):
                errors.append(float('inf'))
                continue
            a, b = original[:hi], decoded[shift:shift+hi]
            errors.append(float(np.mean((a-b)**2) / max(float(np.mean(a*a)), 1e-9)))
        best = int(np.argmin(errors))
        results.append({"sampleRate":info["audioSampleRate"], "channels":channels, "inputSamples":samples,
                        "decodedSamples":len(decoded), "firstPacketTimestampUs":case["audio"][0]["timestamp"],
                        "bestLagSamples":best, "normalizedAlignedError":errors[best], "normalizedZeroLagError":errors[0]})
    result = {"format":"musteroffice.browser-aac-delay-observation/1", "browser":probe["browser"],
              "probeSha256":hashlib.sha256(raw).hexdigest(), "cases":results,
              "scope":"Observed 0..4096 sample waveform alignment on this exact device/profile. Not a cross-platform delay or implementation compensation constant."}
    (args.output / "report.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(results))


if __name__ == "__main__":
    main()
