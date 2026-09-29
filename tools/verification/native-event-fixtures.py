"""Node-edge/default-condition corpus through the real native worker.

Exact start and scale expectations are independent rational arithmetic. Source
variants keep named node targets; missing targets are not inferred. Shared-kernel
parity does not prove Office/WPS save fidelity.
"""

import argparse
import copy
import importlib.util
import json
import subprocess
import xml.etree.ElementTree as X
import zipfile
from fractions import Fraction
from pathlib import Path

from lxml import etree

spec = importlib.util.spec_from_file_location(
    "fixture", Path(__file__).with_name("scale-playback-fixtures.py")
)
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)


def cli(path, args, request=None):
    result = subprocess.run(
        [str(path), *map(str, args)],
        input=None if request is None else json.dumps(request).encode(),
        capture_output=True,
        env={},
        timeout=60,
        check=False,
    )
    assert result.returncode == 0 and not result.stderr, result.stderr
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ["output", "worker", "cli", "author", "schema"]:
        parser.add_argument("--" + key, type=Path, required=True)
    args = parser.parse_args()
    out = args.output
    out.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): f.entry(p) for p in [args.worker, args.cli, args.author]}
    empty = f.put(out / "empty.bin", b"")
    viewport = {
        "width": 800,
        "height": 450,
        "origin": {"x": "0", "y": "0"},
        "scale": {"numerator": 1, "denominator": 10000},
        "coordinateTolerance": "16777216",
        "background": [255] * 4,
    }
    records = {"author": [], "source": []}
    checks = []
    xsd_checks = []
    schema = etree.XMLSchema(etree.parse(str(args.schema)))

    def render(kind, name, request, source=None):
        response, metadata, pixels = f.native(
            args.worker,
            request,
            f.load(source) if source else None,
            b"" if source else None,
        )
        stem = out / name
        record = {
            "name": name,
            "request": f.put(stem.with_suffix(".request.json"), request),
            "response": f.put(stem.with_suffix(".response.json"), metadata),
            "pixels": f.put(stem.with_suffix(".rgba"), pixels),
        }
        if source:
            record.update(source=source, fonts=empty)
        records[kind].append(record)
        state = (
            response["info"]["frame"]["state"]
            if kind == "author"
            else response["info"]["playback"]["evaluated"]["state"]
        )
        return state, pixels

    for event, start in [("begin", Fraction(5, 4)), ("end", Fraction(13, 4))]:
        authored = json.loads(args.author.read_text())
        doc = authored["document"]
        slide = doc["slideOrder"][0]
        objects = doc["slides"][slide]["objects"][:2]
        nodes = [
            {
                "id": "event:" + str(i),
                "restart": "never",
                "start": {"kind": "at", "offset": f.time(1)},
                "duration": f.time(2),
                "endConditions": [],
                "repeatMilli": 1000,
                "repeatDuration": None,
                "fill": "hold",
                "timeTransform": None,
                "effect": dict(
                    kind="scale",
                    target=target,
                    **{
                        "from": {"x": 100000, "y": 100000},
                        "to": {"x": 200000, "y": 200000},
                    },
                ),
            }
            for i, target in enumerate(objects)
        ]
        nodes[1]["start"] = {
            "kind": "after",
            "node": nodes[0]["id"],
            "event": event,
            "delay": f.time(1, 4),
        }
        doc["timelines"] = {
            slide: {"format": "musteroffice.timeline/0.1-draft", "nodes": nodes}
        }
        export_request = f.put(out / (event + ".export.json"), authored)
        pptx = out / (event + ".pptx")
        f.put(
            out / (event + ".export-response.json"),
            cli(args.cli, ["pptx-export", export_request["path"], empty["path"], pptx]),
        )
        snapshot = cli(args.cli, [], {"operation": "initialize", "document": doc})[
            "snapshot"
        ]
        author = {
            "playback": {
                "snapshot": snapshot,
                "slide": slide,
                "binding": {
                    "session": event,
                    "revision": snapshot["revision"],
                    "generation": "1",
                },
                "at": None,
                "history": None,
            },
            "viewport": viewport,
            "defaults": {
                "themeColors": {},
                "pageBackground": {"red": 255, "green": 255, "blue": 255, "alpha": 255},
            },
        }
        with zipfile.ZipFile(pptx) as package:
            parts = {
                p.filename: (p, package.read(p.filename)) for p in package.infolist()
            }
        xml = X.fromstring(parts["ppt/slides/slide1.xml"][1])
        native_objects = {
            o.get("name"): "sp." + o.get("id")
            for o in xml.findall(".//p:cNvPr", f.NS)
            if o.get("name")
        }
        assert len(xml.findall(f'.//p:cond[@evt="{event}"]', f.NS)) == 1
        variants = [f.entry(pptx)]
        for variant in ["target-event", "defaults"]:
            changed = copy.deepcopy(xml)
            for cond in changed.findall(".//p:cond", f.NS):
                if variant == "target-event" and cond.get("evt") == event:
                    cond.set("evt", "on" + event.capitalize())
                if variant == "defaults" and cond.get("delay") == "0":
                    del cond.attrib["delay"]
            output = out / (event + "-" + variant + ".pptx")
            with zipfile.ZipFile(output, "x") as package:
                for name, (info, data) in parts.items():
                    package.writestr(
                        info,
                        X.tostring(changed)
                        if name == "ppt/slides/slide1.xml"
                        else data,
                    )
            variants.append(f.entry(output))
        # Validate the fixture itself before accepting any rendering result.
        # Implementation-note defaults must not invent a serialized enum value.
        for source in variants:
            with zipfile.ZipFile(source["path"]) as package:
                for name in package.namelist():
                    if name.endswith(".xml"):
                        tree = etree.fromstring(package.read(name))
                        if etree.QName(tree).namespace == f.P:
                            schema.assertValid(tree)
                            xsd_checks.append({"source": source, "part": name})
        for index, at in enumerate(
            [
                Fraction(0),
                Fraction(1),
                start - Fraction(1, 1000),
                start,
                start + Fraction(1, 1000),
                start + Fraction(1, 2),
                start + 1,
                start + 2,
                start + 3,
                start + Fraction(1, 2),
            ]
        ):
            author["playback"]["at"] = f.time(at.numerator, at.denominator)
            state, pixels = render("author", f"{event}-author-{index}", author)
            assert state["nodes"][1]["start"] == f.exact(start)
            if at < start:
                assert objects[1] not in state.get("scales", {})
            else:
                scale = f.exact(100000 * (1 + min(at - start, Fraction(2)) / 2))
                assert state["scales"][objects[1]] == {"x": scale, "y": scale}
            for variant, source in enumerate(variants):
                binding = {
                    "session": event,
                    "revision": source["sha256"],
                    "generation": "1",
                }
                page = {
                    "profile": "drawingml-resource-page-q32-v1-draft",
                    "page": {
                        "expectedSourceSha256": source["sha256"],
                        "slide": "/ppt/slides/slide1.xml",
                        "profile": "drawingml-static-solid-page-v1-draft",
                        "colorContext": {"systemColors": {}, "placeholder": None},
                        "viewport": viewport,
                    },
                    "imageSource": "embeddedSnapshot",
                    "sampling": "nearest",
                    "fonts": None,
                }
                request = {
                    "page": page,
                    "sample": {
                        "binding": binding,
                        "at": author["playback"]["at"],
                        "history": None,
                    },
                }
                native_state, native_pixels = render(
                    "source", f"{event}-{variant}-{index}", request, source
                )
                assert pixels == native_pixels, (event, variant, at)
                assert {
                    native_objects[k]: v for k, v in state.get("scales", {}).items()
                } == native_state.get("scales", {})
                for a, b in zip(state["nodes"], native_state["nodes"], strict=True):
                    assert {k: v for k, v in a.items() if k != "node"} == {
                        k: v for k, v in b.items() if k != "node"
                    }
            checks.append(
                {
                    "event": event,
                    "at": f.exact(at),
                    "start": f.exact(start),
                    "pixelSha256": f.sha(pixels),
                }
            )
    for kind, cases in records.items():
        f.put(out / (kind + ".json"), {"cases": cases})
    for record in inputs.values():
        f.load(record)
    report = {
        "status": "passed",
        "inputs": inputs,
        "frameCount": sum(map(len, records.values())),
        "independentChecks": checks,
        "xsdChecks": xsd_checks,
        "scope": "Explicit node lifecycle edges and zero/default condition spelling. No Office/WPS roundtrip acceptance.",
    }
    f.put(out / "report.json", report)
    print(json.dumps({"status": "passed", "frameCount": report["frameCount"]}))


if __name__ == "__main__":
    main()
