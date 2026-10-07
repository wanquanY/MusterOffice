"""Adversarial sync metadata and missing-IDR corpus; never production parsing."""
import json
from pathlib import Path
import struct
import subprocess
import sys
from video_time_fixtures import boxes, box

source, output = Path(sys.argv[1]), Path(sys.argv[2])
output.mkdir(exist_ok=False)
data = source.read_bytes()
changed = []


def tree(payload, active=False):
    result = []
    for kind, content in boxes(payload):
        if kind == b"trak":
            mdia = next(p for k, p in boxes(content) if k == b"mdia")
            handler = next(p for k, p in boxes(mdia) if k == b"hdlr")
            content = tree(content, handler[8:12] == b"vide")
        elif kind == b"moov" or active and kind in (b"mdia", b"minf", b"stbl"):
            content = tree(content, active)
        elif active and kind == b"stss":
            content = bytearray(content)
            assert struct.unpack_from(">I", content, 4)[0] >= 2
            assert struct.unpack_from(">I", content, 8)[0] == 1
            assert struct.unpack_from(">I", content, 12)[0] > 2
            # The second access unit is dependent. Falsely flag it as sync.
            struct.pack_into(">I", content, 12, 2)
            changed.append(True)
        result.append(box(kind, content))
    return b"".join(result)


false_key = output / "false-sync.mp4"
false_key.write_bytes(tree(data))
assert changed and len(false_key.read_bytes()) == len(data)
assert [p for k, p in boxes(data) if k == b"mdat"] == [p for k, p in boxes(false_key.read_bytes()) if k == b"mdat"]
reference = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-ignore_editlist", "1", "-select_streams", "v:0",
                                               "-show_packets", "-of", "json", str(source)]))
first = reference["packets"][0]
at, end = int(first["pos"]), int(first["pos"]) + int(first["size"])
broken = bytearray(data)
removed = 0
while at < end:
    length, = struct.unpack_from(">I", broken, at)
    assert length > 0 and at + 4 + length <= end, "fixture requires 4-byte AVC NAL lengths"
    at += 4
    if broken[at] & 31 == 5:
        broken[at] = (broken[at] & 0xe0) | 1
        removed += 1
    at += length
assert at == end and removed
missing = output / "missing-idr.mp4"
missing.write_bytes(broken)
open_gop = output / "open-gop.mp4"
subprocess.run(["ffmpeg", "-v", "error", "-i", str(source), "-map", "0:v:0", "-an", "-fps_mode", "passthrough",
                "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p",
                "-x264-params", "open-gop=1:keyint=24:min-keyint=24:scenecut=0", str(open_gop)], check=True)
encoded = open_gop.read_bytes()
packets = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-ignore_editlist", "1", "-select_streams", "v:0",
                                              "-show_packets", "-of", "json", str(open_gop)]))["packets"]
non_idr = 0
for packet in packets:
    if "K" not in packet["flags"]:
        continue
    at, end = int(packet["pos"]), int(packet["pos"]) + int(packet["size"])
    idr = False
    while at < end:
        length, = struct.unpack_from(">I", encoded, at)
        assert length > 0 and at + 4 + length <= end
        at += 4
        idr |= encoded[at] & 31 == 5
        at += length
    non_idr += not idr
assert non_idr > 0, "open GOP corpus must actually contain non-IDR sync candidates"
print(json.dumps({"falseSync": str(false_key), "missingIdr": str(missing), "openGop": str(open_gop), "nonIdrCandidates": non_idr}))
