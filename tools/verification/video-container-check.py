#!/usr/bin/env python3
"""Compare actual indexed raw packets against an independent FFmpeg installation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, action="append", required=True)
    parser.add_argument("--ffprobe", type=Path, help="explicit independent binary; otherwise resolve ffprobe from PATH")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    ffprobe = args.ffprobe or shutil.which("ffprobe")
    if not ffprobe:
        raise RuntimeError("independent ffprobe is required")
    ffprobe = Path(ffprobe).resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    reports = []
    for i, source in enumerate(args.input):
        source = source.resolve()
        env = dict(os.environ, MO_BMFF_INPUT=str(source), MO_BMFF_OUTPUT=str(output / f"{i}-index.json"))
        command = ["cargo", "test", "--offline", "--locked", "-p", "mo-media-isobmff", "--test", "real_container", "--", "--ignored", "--nocapture"]
        result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output / f"{i}-test.stdout").write_bytes(result.stdout)
        (output / f"{i}-test.stderr").write_bytes(result.stderr)
        result.check_returncode()
        command = [str(ffprobe), "-v", "error", "-ignore_editlist", "1", "-show_packets", "-show_streams",
                   "-show_data_hash", "sha256", "-of", "json", str(source)]
        reference = subprocess.check_output(command, timeout=120)
        (output / f"{i}-reference.json").write_bytes(reference)
        reference = json.loads(reference)
        actual = json.loads((output / f"{i}-index.json").read_text())
        assert len(actual["tracks"]) == len(reference["streams"])
        count = 0
        packet_duration_differences = 0
        for track in actual["tracks"]:
            stream, = [s for s in reference["streams"] if int(s["id"], 0) == track["id"]]
            assert stream["time_base"] == f"1/{track['timescale']}"
            packets = [p for p in reference["packets"] if p["stream_index"] == stream["index"]]
            assert len(packets) == len(track["samples"])
            assert track["decodeDuration"] == int(stream["duration_ts"])
            for j, (sample, packet) in enumerate(zip(track["samples"], packets)):
                for a, b in [("dts", "dts"), ("pts", "pts"), ("offset", "pos"), ("byteLength", "size")]:
                    assert sample[a] == int(packet[b]), (source.name, track["id"], a, sample, packet)
                # FFmpeg 7.1 may fill packet.duration from the codec's nominal
                # timing. stts stores decode deltas, so compare independently
                # observed consecutive DTS and the demuxer's total decode span.
                end = int(packets[j+1]["dts"]) if j+1 < len(packets) else int(stream["duration_ts"])
                assert sample["duration"] == end - int(packet["dts"])
                packet_duration_differences += sample["duration"] != int(packet.get("duration", 0))
                assert sample["sync"] == ("K" in packet["flags"])
                assert sample["sha256"] == packet["data_hash"].split(":", 1)[1].lower()
                count += 1
        reports.append({"input": source.name, "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                        "tracks": len(actual["tracks"]), "packets": count, "bytesRead": actual["bytesRead"],
                        "fileLength": actual["fileLength"], "packetDurationDifferences": packet_duration_differences,
                        "indexSha256": hashlib.sha256((output / f"{i}-index.json").read_bytes()).hexdigest()})
    report = {"status": "passed", "inputs": reports,
              "ffprobe": subprocess.check_output([str(ffprobe), "-version"], text=True).splitlines()[0],
              "scope": "Actual nonfragmented container sample bytes and raw DTS/CTS compared with independent demux. Edit lists are preserved, not applied by this checker; no decoder, continuous coverage, browser or product acceptance claimed."}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
