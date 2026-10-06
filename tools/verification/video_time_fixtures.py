"""Controlled metadata variations over a real, independently remuxed video.

Only for verification. The remux keeps mdat before moov so growing an edit table
cannot silently invalidate media offsets. Compressed samples remain unchanged.
"""
import hashlib
import json
import struct


def boxes(data):
    offset = 0
    while offset < len(data):
        size, kind = struct.unpack_from(">I4s", data, offset)
        assert 8 <= size <= len(data)-offset, "fixture generator requires normal bounded boxes"
        yield kind, data[offset+8:offset+size]
        offset += size
    assert offset == len(data)


def box(kind, payload):
    return struct.pack(">I4s", len(payload)+8, kind)+payload


def variations(path, ffmpeg, ffprobe, output, run):
    original = output/"remuxed.mp4"
    run([ffmpeg, "-v", "error", "-i", path, "-map", "0", "-c", "copy", original], "fixture-remux")
    data = original.read_bytes(); top = list(boxes(data))
    assert [k for k, _ in top].index(b"mdat") < [k for k, _ in top].index(b"moov")
    ref = json.loads(run([ffprobe, "-v", "error", "-ignore_editlist", "1", "-show_packets", "-show_streams", "-of", "json", original], "fixture-raw-packets"))
    stream = next(s for s in ref["streams"] if s["codec_type"] == "video")
    packets = [p for p in ref["packets"] if p["stream_index"] == stream["index"]]
    origin = min(int(p["pts"]) for p in packets)
    last = max(int(p["pts"]) for p in packets)
    assert origin >= 0
    movie = next(payload for kind, payload in top if kind == b"moov")
    mvhd = next(p for k, p in boxes(movie) if k == b"mvhd")
    assert mvhd[0] == 0
    movie_scale = struct.unpack_from(">I", mvhd, 12)[0]
    assert movie_scale % 10 == 0, "explicit fixture movie timescale"
    changed_ctts = []

    def edit_table(mode, payload):
        assert payload[0] == 0
        count, = struct.unpack_from(">I", payload, 4)
        entries = [struct.unpack_from(">IiI", payload, 8+i*12) for i in range(count)]
        if mode == "signed":
            entries = [(d, t-origin if t >= 0 else t, r) for d, t, r in entries]
        else:
            unit = movie_scale//10
            entries = [(unit, -1, 65536), (5*unit, origin, 98304), (4*unit, origin, 65536),
                       (3*unit, origin+(last-origin)//3, 0), (unit, -1, 65536), (3*unit, last+10000, 65536)]
        return b"\0"*4+struct.pack(">I", len(entries))+b"".join(struct.pack(">IiI", *e) for e in entries)

    def tree(payload, mode, path=()):
        result = []
        for kind, p in boxes(payload):
            if kind == b"trak" and not path:
                mdia = next(v for k, v in boxes(p) if k == b"mdia")
                hdlr = next(v for k, v in boxes(mdia) if k == b"hdlr")
                if hdlr[8:12] == b"vide":
                    p = tree(p, mode, (b"trak",))
            elif path:
                if kind in (b"mdia", b"minf", b"stbl", b"edts"):
                    p = tree(p, mode, path+(kind,))
                elif kind == b"tkhd":
                    p = bytearray(p); assert p[0] == 0
                    struct.pack_into(">I", p, 12, 0x80000001 if mode == "signed" else 42)
                elif kind == b"ctts" and mode == "signed":
                    p = bytearray(p); version = p[0]; count, = struct.unpack_from(">I", p, 4); p[0] = 1
                    for i in range(count):
                        at = 12+i*8
                        delta, = struct.unpack_from(">i" if version == 1 else ">I", p, at)
                        struct.pack_into(">i", p, at, delta-origin)
                    changed_ctts.append(True)
                elif kind == b"elst":
                    p = edit_table(mode, p)
            result.append(box(kind, p))
        return b"".join(result)

    paths = []
    for mode in ("signed", "edits"):
        path = output/f"{mode}-timing.mp4"
        path.write_bytes(b"".join(box(k, tree(p, mode) if k == b"moov" else p) for k, p in top))
        paths.append(path)
    assert changed_ctts, "signed composition fixture requires an actual CTTS table"
    # Assert byte-for-byte compressed payload preservation independently of offsets.
    media_hash = [hashlib.sha256(p).hexdigest() for k, p in top if k == b"mdat"]
    for path in paths:
        assert [hashlib.sha256(p).hexdigest() for k, p in boxes(path.read_bytes()) if k == b"mdat"] == media_hash
    return paths
