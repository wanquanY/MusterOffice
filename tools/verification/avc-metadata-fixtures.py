"""Generated AVC metadata corpus, with a mid-stream SPS/PPS update and later
IDRs that omit parameter NALs. No third-party images or videos are downloaded.
Requires an explicit FFmpeg executable and a new output directory.
"""
import argparse
import json
from pathlib import Path
import re
import subprocess


def run(ffmpeg, *args):
    subprocess.run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin", "-n", *map(str, args)], check=True)


def strip_repeated_parameters(data):
    starts = list(re.finditer(b"\x00\x00(?:\x00)?\x01", data))
    if not starts or starts[0].start() != 0:
        raise ValueError("expected Annex B")
    result, seen = [], set()
    for i, start in enumerate(starts):
        end = starts[i + 1].start() if i + 1 < len(starts) else len(data)
        kind = data[start.end()] & 31
        if kind in (7, 8):
            if kind in seen:
                continue
            seen.add(kind)
        result.append(data[start.start():end])
    if seen != {7, 8}:
        raise ValueError("missing parameter sets")
    return b"".join(result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ffmpeg")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    segments = []
    for location, cabac in [(0, 1), (1, 0)]:
        segment = output / f"segment-{location}.h264"
        run(args.ffmpeg, "-f", "lavfi", "-i", "testsrc2=size=128x96:rate=12", "-t", "1", "-an",
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "4", "-bf", "0",
            "-color_primaries", "bt709", "-color_trc", "bt709", "-colorspace", "bt709",
            "-x264-params", f"chromaloc={location}:cabac={cabac}:repeat-headers=1", "-f", "h264", segment)
        segments.append(strip_repeated_parameters(segment.read_bytes()))
    elementary = output / "changed.h264"
    elementary.write_bytes(b"".join(segments))
    changed = output / "changed.mp4"
    run(args.ffmpeg, "-fflags", "+genpts", "-r", "12", "-f", "h264", "-i", elementary,
        "-c:v", "copy", "-tag:v", "avc3", changed)
    fixtures = [changed]
    for location in range(6):
        target = output / f"location-{location}.mp4"
        run(args.ffmpeg, "-i", changed, "-map", "0:v:0", "-c:v", "copy",
            "-bsf:v", f"h264_metadata=chroma_sample_loc_type={location}", "-tag:v", "avc3", target)
        fixtures.append(target)
    print(json.dumps({"fixtures": list(map(str, fixtures))}))


if __name__ == "__main__":
    main()
