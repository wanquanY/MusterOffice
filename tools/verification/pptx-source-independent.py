"""Verify real native edit outputs with independent ZIP/XML implementations.

The corpus edits ordinary text only. Unchanged entries must keep their exact
compressed payloads; changed parts must match the requested leaf substitution
byte-for-byte, including UTF-16 encoding and all unknown source markup.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import zipfile
from mce_reference import project

P = "http://schemas.openxmlformats.org/presentationml/2006/main"
A = "http://schemas.openxmlformats.org/drawingml/2006/main"
NS = {"p": P, "a": A}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def compressed(raw, info):
    fields = struct.unpack_from("<IHHHHHIIIHH", raw, info.header_offset)
    assert fields[0] == 0x04034B50
    start = info.header_offset + 30 + fields[-2] + fields[-1]
    return raw[start:start + info.compress_size]


def escape(text):
    return text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\r", "&#xD;")


def text_node(root, target):
    objects = [node for node in root.findall(".//p:sp", NS)
               if node.find("p:nvSpPr/p:cNvPr", NS).get("id") == str(target["objectId"])]
    assert len(objects) == 1
    paragraphs = objects[0].findall("p:txBody/a:p", NS)
    runs = [node for node in paragraphs[target["paragraph"]] if node.tag in [f"{{{A}}}r", f"{{{A}}}fld", f"{{{A}}}br"]]
    assert runs[target["run"]].tag == f"{{{A}}}r"
    return runs[target["run"]].find("a:t", NS)


def verify(item):
    source = Path(item["source"]).read_bytes(); output = Path(item["output"]).read_bytes()
    request = json.loads(Path(item["request"]).read_text())
    assert sha(source) == request["expectedSourceSha256"]
    assert sha(output) == item["sha256"]
    if item["noop"]:
        assert output == source
    edits = {}
    for edit in request["edits"]:
        if edit["expectedText"] != edit["replacement"]:
            edits.setdefault(edit["target"]["part"].lstrip("/"), []).append(edit)
    preserved = 0
    with zipfile.ZipFile(item["source"]) as before, zipfile.ZipFile(item["output"]) as after:
        assert before.testzip() is None and after.testzip() is None
        assert set(before.namelist()) == set(after.namelist())
        for name in before.namelist():
            original = before.read(name); actual = after.read(name)
            if name not in edits:
                assert actual == original, name
                assert compressed(source, before.getinfo(name)) == compressed(output, after.getinfo(name)), name
                preserved += 1
                continue
            source_root, _ = project(original)
            actual_root, _ = project(actual)
            if original.startswith(b"\xff\xfe"):
                bom, encoding = b"\xff\xfe", "utf-16-le"
            elif original.startswith(b"\xfe\xff"):
                bom, encoding = b"\xfe\xff", "utf-16-be"
            else:
                bom, encoding = b"", "utf-8"
            expected_raw = original[len(bom):].decode(encoding)
            for edit in edits[name]:
                old = text_node(source_root, edit["target"])
                new = text_node(actual_root, edit["target"])
                assert old is not None and new is not None
                assert "".join(old.itertext()) == edit["expectedText"]
                assert "".join(new.itertext()) == edit["replacement"]
                # These owned corpus leaves contain ordinary text with one unique
                # spelling; production rewriting instead uses namespace XML spans.
                escaped_old = escape(edit["expectedText"])
                assert expected_raw.count(escaped_old) == 1
                expected_raw = expected_raw.replace(escaped_old, escape(edit["replacement"]), 1)
            assert actual == bom + expected_raw.encode(encoding), f"unexpected source markup change: {name}"
    return {"name": item["name"], "result": "passed", "sourceSha256": sha(source), "outputSha256": sha(output),
            "noopByteIdentity": item["noop"], "changedParts": len(edits), "unchangedCompressedEntriesVerified": preserved}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity_report", type=Path)
    args = parser.parse_args()
    report = json.loads(args.parity_report.read_text())
    cases = [verify(item) for item in report["outputs"]]
    print(json.dumps({"format": "musteroffice.pptx-source-independent/1", "passed": len(cases), "cases": cases,
                      "scope": "Actual leaf edits and exact preservation; no complete model, layout, rendering or Office/WPS acceptance."}, indent=2))


if __name__ == "__main__":
    main()
