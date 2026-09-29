"""Native appearance calibration and public SDK replay fixtures.

Owned WPS Appear/Disappear samples are compared against independent static
visible/hidden pages. Author/export roundtrips use the same typed timing tree.
This is a bounded fixture verification, not complete Office interoperability.
"""

import argparse
import copy
import hashlib
import io
import json
import subprocess
import zipfile
from pathlib import Path
from xml.etree import ElementTree as X

NS = {"p": "http://schemas.openxmlformats.org/presentationml/2006/main"}
SLIDE = "ppt/slides/slide1.xml"
TIMES = [0, 99, 100, 101, 499, 500, 999, 1000, 1001, 4000, 0, 100, 500]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def record(path):
    data = path.read_bytes()
    return {"path": str(path), "sha256": sha(data), "byteLength": len(data)}


def put(path, value):
    data = value if isinstance(value, bytes) else json.dumps(value).encode()
    with path.open("xb") as stream:
        stream.write(data)
    return record(path)


def time(ms):
    return {"ticks": str(ms), "timescale": 1000}


def history(binding):
    return {
        "binding": binding,
        "through": time(5000),
        "events": [
            {
                "generation": binding["generation"],
                "sequence": i + 1,
                "at": time(ms),
                "event": {"kind": "navigation", "direction": direction, "target": None},
            }
            for i, (ms, direction) in enumerate(
                [(100, "next"), (500, "previous"), (1000, "next")]
            )
        ],
    }


def worker(path, request, source=None, fonts=None, static=False):
    blocks = [json.dumps(request).encode()]
    if source is not None:
        blocks += [source, fonts]
    mode = (
        "--playback-page"
        if source is None
        else ("--pptx-resource-page" if static else "--pptx-playback-page")
    )
    result = subprocess.run(
        [str(path), mode],
        check=False,
        env={},
        capture_output=True,
        timeout=60,
        input=b"".join(len(b).to_bytes(4, "little") for b in blocks) + b"".join(blocks),
    )
    if result.returncode or result.stderr:
        raise ValueError(result.stderr.decode(errors="replace"))
    ml, pl = [int.from_bytes(result.stdout[i : i + 4], "little") for i in (0, 4)]
    assert len(result.stdout) == 8 + ml + pl
    response = json.loads(result.stdout[8 : 8 + ml])
    assert response["status"] == "rendered", response
    return response, result.stdout[8 + ml :]


def cli(path, args, request=None):
    result = subprocess.run(
        [str(path), *map(str, args)],
        check=False,
        env={},
        capture_output=True,
        timeout=60,
        input=None if request is None else json.dumps(request).encode(),
    )
    if result.returncode or result.stderr:
        raise ValueError(result.stderr.decode(errors="replace"))
    return json.loads(result.stdout)


def control(raw, hidden):
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        entries = archive.infolist()
        parts = {n: archive.read(n) for n in archive.namelist()}
    root = X.fromstring(parts[SLIDE])
    root.remove(root.find("p:timing", NS))
    target = root.find(".//p:cNvPr[@id='2']", NS)
    assert target is not None
    target.set("hidden", "1" if hidden else "0")
    parts[SLIDE] = X.tostring(root)
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
        for entry in entries:
            archive.writestr(entry, parts[entry.filename])
    return output.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in (
        "output",
        "worker",
        "cli",
        "appear",
        "disappear",
        "prepare",
        "fonts",
        "author",
    ):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): record(p) for k, p in vars(args).items() if k != "output"}
    sources, authors = [], []
    prep = json.loads(args.prepare.read_text())
    prep["page"]["page"]["viewport"].update(
        width=800, height=450, scale={"numerator": 1, "denominator": 10000}
    )
    fonts = args.fonts.read_bytes()

    def save(kind, name, request, response, pixels, source=None):
        stem = args.output / name
        case = {
            "name": name,
            "request": put(stem.with_suffix(".request.json"), request),
            "response": put(stem.with_suffix(".response.json"), response),
            "pixels": put(stem.with_suffix(".rgba"), pixels),
        }
        if source is not None:
            case.update(source=record(source), fonts=record(args.fonts))
        (authors if kind == "author" else sources).append(case)

    for name in ("appear", "disappear"):
        source = getattr(args, name)
        raw = source.read_bytes()
        page = copy.deepcopy(prep["page"])
        page["page"]["expectedSourceSha256"] = sha(raw)
        binding = {"session": name, "revision": sha(raw), "generation": "0"}
        controls = {}
        for hidden in (False, True):
            static = control(raw, hidden)
            path = args.output / f"{name}-static-{hidden}.pptx"
            put(path, static)
            q = copy.deepcopy(page)
            q["page"]["expectedSourceSha256"] = sha(static)
            response, pixels = worker(args.worker, q, static, fonts, static=True)
            put(path.with_suffix(".request.json"), q)
            put(path.with_suffix(".response.json"), response)
            put(path.with_suffix(".rgba"), pixels)
            controls[hidden] = pixels
        assert controls[False] != controls[True]
        for i, ms in enumerate(TIMES):
            q = {
                "page": page,
                "sample": {
                    "binding": binding,
                    "at": time(ms),
                    "history": history(binding),
                },
            }
            response, pixels = worker(args.worker, q, raw, fonts)
            active = 100 <= ms < 500 or ms >= 1000
            assert pixels == controls[active if name == "disappear" else not active], (
                name,
                ms,
            )
            save("source", f"{name}-native-{i}", q, response, pixels, source)
        query = args.output / f"{name}-timing.request.json"
        put(query, {"expectedSourceSha256": sha(raw), "slide": "/" + SLIDE})
        inspected = cli(args.cli, ["pptx-timing", query, source])
        assert inspected["status"] == "inspected", inspected
        put(query.with_name(f"{name}-timing.response.json"), inspected)
        timeline = inspected["timing"]["native"]["timeline"]
        document = json.loads(args.author.read_text())["document"]
        slide = document["slideOrder"][0]
        target = document["slides"][slide]["objects"][0]
        assert len(timeline["nodes"]) == 1
        timeline["nodes"][0]["effect"]["target"] = target
        document["timelines"] = {slide: timeline}
        initialized = cli(
            args.cli, [], {"operation": "initialize", "document": document}
        )
        snapshot = initialized["snapshot"]
        author_binding = {
            "session": name + "-author",
            "revision": snapshot["revision"],
            "generation": "0",
        }
        defaults = json.loads(args.author.read_text())["defaults"]
        export = args.output / f"{name}-export.json"
        put(
            export, {"document": document, "defaults": defaults, "resourceBindings": []}
        )
        empty = args.output / f"{name}-resources.bin"
        put(empty, b"")
        pptx = args.output / f"{name}-export.pptx"
        exported = cli(args.cli, ["pptx-export", export, empty, pptx])
        put(export.with_name(f"{name}-export.response.json"), exported)
        export_raw = pptx.read_bytes()
        export_page = copy.deepcopy(page)
        export_page["page"]["expectedSourceSha256"] = sha(export_raw)
        export_binding = {
            "session": name + "-export",
            "revision": sha(export_raw),
            "generation": "0",
        }
        for i, ms in enumerate(TIMES):
            aq = {
                "playback": {
                    "snapshot": snapshot,
                    "slide": slide,
                    "binding": author_binding,
                    "at": time(ms),
                    "history": history(author_binding),
                },
                "viewport": page["page"]["viewport"],
                "defaults": {
                    key: defaults[key] for key in ("themeColors", "pageBackground")
                },
            }
            response, pixels = worker(args.worker, aq)
            active = 100 <= ms < 500 or ms >= 1000
            assert pixels == controls[active if name == "disappear" else not active], (
                name,
                ms,
                "author",
            )
            save("author", f"{name}-author-{i}", aq, response, pixels)
            eq = {
                "page": export_page,
                "sample": {
                    "binding": export_binding,
                    "at": time(ms),
                    "history": history(export_binding),
                },
            }
            eresponse, epixels = worker(args.worker, eq, export_raw, fonts)
            assert epixels == pixels, (name, ms, "export")
            save("source", f"{name}-export-{i}", eq, eresponse, epixels, pptx)
    put(args.output / "author.json", {"cases": authors})
    put(args.output / "source.json", {"cases": sources})
    assert all(record(Path(p)) == value for p, value in inputs.items()), "input changed"
    report = {
        "status": "passed",
        "inputs": inputs,
        "sourceFrames": len(sources),
        "authorFrames": len(authors),
        "staticControls": 4,
        "scope": __doc__,
    }
    put(args.output / "report.json", report)
    print(json.dumps({k: v for k, v in report.items() if k not in ("inputs", "scope")}))


if __name__ == "__main__":
    main()
