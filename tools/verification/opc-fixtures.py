"""Create independent Python ZIP/XML inputs and verify Rust-produced packages.

All content is synthetic; no PowerPoint renderer or Office conformance claim.
Output is deterministic on a fixed Python/zlib version. The manifest records hashes.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct
import zipfile
import xml.etree.ElementTree as ET

CT = "http://schemas.openxmlformats.org/package/2006/content-types"
RELS = "http://schemas.openxmlformats.org/package/2006/relationships"
TYPES = f'<Types xmlns="{CT}"><Default Extension="xml" ContentType="application/xml"/><Default Extension="bin" ContentType="application/octet-stream"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>'.encode()
ROOT_RELS = f'<Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="urn:fixture:main" Target="doc/main.xml"/></Relationships>'.encode()
MAIN = '<x:d xmlns:x="urn:fixture" xmlns:f="urn:future" f:keep="yes"><!--keep--><x:t>中文 🚀 é مرحبا</x:t></x:d>'.encode()
PAYLOAD = bytes(range(256)) * 512
ENTRIES = [("[Content_Types].xml", TYPES), ("_rels/.rels", ROOT_RELS), ("doc/main.xml", MAIN), ("media/payload.bin", PAYLOAD)]


class StreamingBuffer(io.BytesIO):
    def seek(self, *args):
        raise io.UnsupportedOperation("non-seekable fixture writer")


def make_zip(entries=ENTRIES, compression=zipfile.ZIP_DEFLATED, stream=False, zip64=False):
    buffer = StreamingBuffer() if stream else io.BytesIO()
    with zipfile.ZipFile(buffer, "w", compression=compression, compresslevel=6) as archive:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
            info.compress_type = compression
            info.external_attr = 0o100644 << 16
            with archive.open(info, "w", force_zip64=zip64) as entry:
                entry.write(data)
    return buffer.getvalue()


def expected_report(data):
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        assert archive.testzip() is None
        types = ET.fromstring(archive.read("[Content_Types].xml"))
        defaults = {e.attrib["Extension"].lower(): e.attrib["ContentType"] for e in types if e.tag == f"{{{CT}}}Default"}
        overrides = {e.attrib["PartName"].lower(): e.attrib["ContentType"] for e in types if e.tag == f"{{{CT}}}Override"}
        parts = []
        for info in archive.infolist():
            if info.filename == "[Content_Types].xml":
                continue
            content = archive.read(info)
            name = "/" + info.filename
            content_type = overrides.get(name.lower()) or defaults[info.filename.rsplit(".", 1)[-1].lower()]
            if content_type.endswith("xml"):
                ET.fromstring(content)
            parts.append({"name": name, "contentType": content_type, "byteLength": str(len(content)), "compressedLength": str(info.compress_size), "sha256": hashlib.sha256(content).hexdigest()})
        return {"sha256": hashlib.sha256(data).hexdigest(), "byteLength": str(len(data)), "parts": sorted(parts, key=lambda p: p["name"].lower())}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    directory = args.directory
    directory.mkdir(parents=True, exist_ok=True)
    cases = []

    def add(name, data, code=None):
        path = directory / f"{name}.opc"
        path.write_bytes(data)
        case = {"name": name, "file": path.name, "sha256": hashlib.sha256(data).hexdigest(), "status": "error" if code else "inspected"}
        if code:
            case["code"] = code
        else:
            case["expected"] = expected_report(data)
        cases.append(case)

    add("python-stored", make_zip(compression=zipfile.ZIP_STORED))
    add("python-deflated", make_zip())
    add("python-streamed", make_zip(stream=True))
    add("python-streamed-zip64", make_zip(stream=True, zip64=True))
    add("python-utf16", make_zip([(n, ('<?xml version="1.0" encoding="UTF-16"?>' + d.decode()).encode("utf-16") if n.endswith("xml") else d) for n, d in ENTRIES]))
    escaped = "doc/%E4%B8%AD.xml"
    add("python-escaped-uri", make_zip([(n.replace("doc/main.xml", escaped), d.replace(b"doc/main.xml", escaped.encode()) if n == "_rels/.rels" else d) for n, d in ENTRIES]))
    add("not-zip", b"not a ZIP file", "INPUT_INVALID")
    base = make_zip(compression=zipfile.ZIP_STORED)
    add("truncated", base[:-5], "INPUT_INVALID")
    add("trailing-junk", base + b"junk", "INPUT_INVALID")
    corrupt = bytearray(base)
    corrupt[corrupt.index(MAIN) + 5] ^= 1
    add("bad-crc", corrupt, "INPUT_INVALID")
    add("missing-target", make_zip([e for e in ENTRIES if e[0] != "doc/main.xml"]), "INPUT_INVALID")
    add("equivalent-part", make_zip(ENTRIES + [("DOC/MAIN.XML", MAIN)]), "INPUT_INVALID")
    add("traversal", make_zip(ENTRIES + [("../evil.xml", MAIN)]), "INPUT_INVALID")
    add("malformed-xml", make_zip([(n, b"<a><b/></x>" if n == "doc/main.xml" else d) for n, d in ENTRIES]), "INPUT_INVALID")
    add("dtd", make_zip([(n, b'<!DOCTYPE a [<!ENTITY x SYSTEM "file:///never-read">]><a>&x;</a>' if n == "doc/main.xml" else d) for n, d in ENTRIES]), "INPUT_INVALID")
    add("xml-depth-budget", make_zip([(n, b"<a>" * 257 + b"</a>" * 257 if n == "doc/main.xml" else d) for n, d in ENTRIES]), "LIMIT_EXCEEDED")
    descriptor = bytearray(make_zip(stream=True))
    end = descriptor.index(b"PK\x07\x08")
    descriptor[end + 4] ^= 1
    add("descriptor-crc", descriptor, "INPUT_INVALID")
    # Central directory's claimed count must not suppress entries.
    count = bytearray(base)
    struct.pack_into("<HH", count, len(count) - 22 + 8, 0, 0)
    add("false-entry-count", count, "INPUT_INVALID")

    built = (directory / "kernel-built.opc").read_bytes()
    copied = (directory / "kernel-copy.opc").read_bytes()
    edited = (directory / "kernel-edited.opc").read_bytes()
    assert copied == built, "no-op rewrite changed source bytes"
    with zipfile.ZipFile(io.BytesIO(built)) as source, zipfile.ZipFile(io.BytesIO(edited)) as target:
        assert source.namelist() == target.namelist()
        for name in source.namelist():
            if name == "doc/main.xml":
                assert source.read(name) != target.read(name)
                assert target.read(name) == source.read(name).replace("中文 🚀 é".encode(), b"changed")
                assert ET.fromstring(target.read(name)).find("{urn:fixture}t").text == "changed"
            else:
                assert source.read(name) == target.read(name), name
    for name in ["kernel-built", "kernel-copy", "kernel-edited"]:
        add(name, (directory / f"{name}.opc").read_bytes())
    (directory / "manifest.json").write_text(json.dumps({"format": "musteroffice.opc-fixtures/1", "independentWriter": "Python zipfile", "independentVerification": "Python zipfile CRC + ElementTree + SHA-256 + unknown-part comparison", "cases": cases}, indent=2) + "\n")
    print(json.dumps({"fixtures": len(cases), "independentChecks": "passed"}))


if __name__ == "__main__":
    main()
