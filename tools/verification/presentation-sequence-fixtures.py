"""Exercise semantic authoring through the public atomic edit and native export.

Owned six-rectangle documents only. Independent exact intervals and real WPS
edit observations are distinct from native/WASM shared-kernel parity.
"""

import argparse
import importlib.util
import json
import xml.etree.ElementTree as X
import zipfile
from fractions import Fraction
from pathlib import Path

from lxml import etree

spec = importlib.util.spec_from_file_location(
    "node_fixtures", Path(__file__).with_name("native-event-fixtures.py")
)
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
f, cli = common.f, common.cli


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ["output", "worker", "cli", "author", "schema", "wps-evidence"]:
        parser.add_argument("--" + key, type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {
        str(p): f.entry(p) for p in [args.worker, args.cli, args.author, Path(__file__)]
    }
    empty = f.put(out / "empty.bin", b"")
    schema = etree.XMLSchema(etree.parse(str(args.schema)))
    records, xsd, checks = {"author": [], "source": []}, [], []
    viewport = {
        "width": 800,
        "height": 450,
        "origin": {"x": "0", "y": "0"},
        "scale": {"numerator": 1, "denominator": 10000},
        "coordinateTolerance": "16777216",
        "background": [255] * 4,
    }

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

    def validate(source):
        with zipfile.ZipFile(source["path"]) as package:
            for name in package.namelist():
                if name.endswith(".xml"):
                    tree = etree.fromstring(package.read(name))
                    if etree.QName(tree).namespace == f.P:
                        schema.assertValid(tree)
                        xsd.append({"source": source, "part": name})
            xml = X.fromstring(package.read("ppt/slides/slide1.xml"))
        return {
            o.get("name"): "sp." + o.get("id")
            for o in xml.findall(".//p:cNvPr", f.NS)
            if o.get("name")
        }

    variants = [
        ("native-original", 1, 1, "native-after-previous.pptx"),
        ("native-duration", 2, 1, "kernel-export-wps-edited.pptx"),
        ("parallel", 2, 2, "kernel-export-wps-overlap.pptx"),
        ("author-edited", 3, 2, "compiled-sequence-wps.pptx"),
    ]
    for name, longest, parallel_count, wps_file in variants:
        authored = json.loads(args.author.read_bytes())
        doc = authored["document"]
        doc.pop("timelines", None)
        slide = doc["slideOrder"][0]
        objects = doc["slides"][slide]["objects"]
        assert len(objects) == 6
        effects = [
            {
                "id": "effect:" + str(i),
                "delay": f.time(0),
                "duration": f.time(longest if i == 0 else 1),
                "repeatMilli": 1000,
                "fill": "hold",
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
        sequence = {
            "groups": [
                {
                    "start": "automatic",
                    "batches": [
                        {"delay": f.time(0), "effects": effects[:parallel_count]},
                        {"delay": f.time(0), "effects": effects[parallel_count:]},
                    ],
                }
            ]
        }
        snapshot = cli(args.cli, [], {"operation": "initialize", "document": doc})[
            "snapshot"
        ]
        request = {
            "operation": "prepare",
            "snapshot": snapshot,
            "transaction": {
                "documentId": doc["id"],
                "requestId": "sequence:" + name,
                "baseRevision": snapshot["revision"],
                "operations": [
                    {
                        "operationId": "animation",
                        "operation": {
                            "kind": "setPresentationSequence",
                            "slide": slide,
                            "sequence": sequence,
                        },
                    }
                ],
            },
        }
        f.put(out / (name + ".edit-request.json"), request)
        response = cli(args.cli, [], request)
        assert response["status"] == "prepared", response
        f.put(out / (name + ".edit-response.json"), response)
        snapshot = response["snapshot"]
        authored["document"] = snapshot["document"]
        export = f.put(out / (name + ".export.json"), authored)
        pptx = out / (name + ".pptx")
        result = cli(args.cli, ["pptx-export", export["path"], empty["path"], pptx])
        f.put(out / (name + ".export-response.json"), result)
        native = f.entry(pptx)
        sources = [(native, validate(native), True)]
        if wps_file:
            source = f.entry(args.wps_evidence / wps_file)
            inputs[source["path"]] = source
            # WPS baseline inherited shape:0..5 names from the owned fixture.
            names = validate(source)
            sources.append((source, names, False))
        binding = {"session": name, "revision": snapshot["revision"], "generation": "1"}
        history = {"binding": binding, "through": f.time(10), "events": []}
        for index, ms in enumerate(
            [0, 500, 999, 1000, 1500, 2000, 2500, 3000, 3500, 4500, 750]
        ):
            at = Fraction(ms, 1000)
            request = {
                "playback": {
                    "snapshot": snapshot,
                    "slide": slide,
                    "binding": binding,
                    "at": f.time(ms, 1000),
                    "history": history,
                },
                "viewport": viewport,
                "defaults": {
                    "themeColors": {},
                    "pageBackground": {
                        "red": 255,
                        "green": 255,
                        "blue": 255,
                        "alpha": 255,
                    },
                },
            }
            state, pixels = render("author", f"{name}-author-{index}", request)
            expected = {}
            for i, obj in enumerate(objects):
                start = Fraction(0 if i < parallel_count else longest)
                duration = Fraction(longest if i == 0 else 1)
                if at >= start:
                    scale = f.exact(
                        100000 * (1 + min((at - start) / duration, Fraction(1)))
                    )
                    expected[obj] = {"x": scale, "y": scale}
            assert state.get("scales", {}) == expected, (name, ms, state)
            for variant, (source, names, equal_pixels) in enumerate(sources):
                b = {"session": name, "revision": source["sha256"], "generation": "1"}
                query = {
                    "page": {
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
                    },
                    "sample": {
                        "binding": b,
                        "at": f.time(ms, 1000),
                        "history": {"binding": b, "through": f.time(10), "events": []},
                    },
                }
                native_state, native_pixels = render(
                    "source", f"{name}-{variant}-{index}", query, source
                )
                # The original WPS control calls its shapes shape0..5; kernel
                # exports use shape:0..5. Explicit native IDs are read per file.
                mapping = {
                    obj: names.get(obj, names.get(obj.replace(":", "")))
                    for obj in objects
                }
                assert all(mapping.values()), names
                assert native_state.get("scales", {}) == {
                    mapping[k]: v for k, v in expected.items()
                }, (name, variant, ms)
                if equal_pixels:
                    assert native_pixels == pixels, (name, ms)
            checks.append(
                {
                    "case": name,
                    "at": f.time(ms, 1000),
                    "expectedScales": expected,
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
        "xsdChecks": xsd,
        "checks": checks,
        "frameCount": sum(map(len, records.values())),
        "scope": "Ordered authoring and WPS owned editing controls; not full Office/WPS acceptance",
    }
    f.put(out / "report.json", report)
    print(json.dumps({"status": "passed", "frameCount": report["frameCount"]}))


if __name__ == "__main__":
    main()
