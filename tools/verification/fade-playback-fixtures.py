"""Owned editable fades: exact timing, native pixels, source resources and XSD.

Native/WASM parity is a separate check. WPS files here were saved from owned
calibration shapes; accepting their timing is not universal Office acceptance.
"""

import argparse
import copy
import importlib.util
import io
import json
import xml.etree.ElementTree as X
import zipfile
from fractions import Fraction
from pathlib import Path

from lxml import etree

spec = importlib.util.spec_from_file_location(
    "common", Path(__file__).with_name("native-event-fixtures.py")
)
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
f, cli = common.f, common.cli


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ["output", "worker", "cli", "author", "schema", "wps", "resources"]:
        parser.add_argument("--" + key, type=Path, required=True)
    a = parser.parse_args()
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {
        str(p): f.entry(p)
        for p in [
            a.worker,
            a.cli,
            a.author,
            a.schema,
            a.wps,
            a.resources,
            Path(__file__),
        ]
    }
    records = {"author": [], "source": []}
    empty = f.put(out / "empty.bin", b"")
    schema = etree.XMLSchema(etree.parse(str(a.schema)))
    validations, checks = [], []

    def validate(source):
        with zipfile.ZipFile(source["path"]) as z:
            for name in z.namelist():
                if name.endswith(".xml"):
                    tree = etree.fromstring(z.read(name))
                    if etree.QName(tree).namespace == f.P:
                        schema.assertValid(tree)
                        validations.append({"source": source, "part": name})
            return X.fromstring(z.read("ppt/slides/slide1.xml"))

    def render(kind, name, q, source=None, fonts=empty):
        result, metadata, pixels = f.native(
            a.worker,
            q,
            f.load(source) if source else None,
            f.load(fonts) if source else None,
        )
        record = {
            "name": name,
            "request": f.put(out / (name + ".request.json"), q),
            "response": f.put(out / (name + ".response.json"), metadata),
            "pixels": f.put(out / (name + ".rgba"), pixels),
        }
        if source:
            record.update(source=source, fonts=fonts)
        records[kind].append(record)
        state = (
            result["info"]["frame"]["state"]
            if kind == "author"
            else result["info"]["playback"]["evaluated"]["state"]
        )
        return state, pixels

    authored = json.loads(a.author.read_bytes())
    doc = authored["document"]
    doc.pop("timelines", None)
    slide = doc["slideOrder"][0]
    objects = doc["slides"][slide]["objects"]
    groups = []
    for i, direction in enumerate(["in", "out"]):
        effect = {
            "id": "fade:" + str(i),
            "delay": f.time(0),
            "duration": f.time(1, 2),
            "repeatMilli": 1000,
            "fill": "remove",
            "effect": {"kind": "fade", "target": objects[i], "transition": direction},
        }
        groups.append(
            {"start": "next", "batches": [{"delay": f.time(0), "effects": [effect]}]}
        )
    snapshot = cli(a.cli, [], {"operation": "initialize", "document": doc})["snapshot"]
    edit = {
        "operation": "prepare",
        "snapshot": snapshot,
        "transaction": {
            "documentId": doc["id"],
            "requestId": "fade-edit",
            "baseRevision": snapshot["revision"],
            "operations": [
                {
                    "operationId": "fade",
                    "operation": {
                        "kind": "setPresentationSequence",
                        "slide": slide,
                        "sequence": {"groups": groups},
                    },
                }
            ],
        },
    }
    f.put(out / "fade.edit-request.json", edit)
    response = cli(a.cli, [], edit)
    assert response["status"] == "prepared", response
    f.put(out / "fade.edit-response.json", response)
    snapshot = response["snapshot"]
    authored["document"] = snapshot["document"]
    export = f.put(out / "fade.export.json", authored)
    pptx = out / "fade.pptx"
    cli(a.cli, ["pptx-export", export["path"], empty["path"], pptx])
    source = f.entry(pptx)
    validate(source)
    wps = f.entry(a.wps)
    validate(wps)
    viewport = {
        "width": 800,
        "height": 450,
        "origin": {"x": "0", "y": "0"},
        "scale": {"numerator": 1, "denominator": 10000},
        "coordinateTolerance": "16777216",
        "background": [255] * 4,
    }
    binding = {"session": "fade", "revision": snapshot["revision"], "generation": "1"}

    def history(b):
        return {
            "binding": b,
            "through": f.time(5),
            "events": [
                {
                    "generation": "1",
                    "sequence": i + 1,
                    "at": f.time(ms, 1000),
                    "event": {
                        "kind": "navigation",
                        "direction": direction,
                        "target": None,
                    },
                }
                for i, (ms, direction) in enumerate(
                    [(100, "next"), (1000, "next"), (2000, "previous")]
                )
            ],
        }

    for i, ms in enumerate(
        [0, 99, 100, 225, 350, 599, 600, 1000, 1125, 1250, 1498, 1499, 1500, 2000, 350]
    ):
        request = {
            "playback": {
                "snapshot": snapshot,
                "slide": slide,
                "binding": binding,
                "at": f.time(ms, 1000),
                "history": history(binding),
            },
            "viewport": viewport,
            "defaults": {
                "themeColors": {},
                "pageBackground": {"red": 255, "green": 255, "blue": 255, "alpha": 255},
            },
        }
        state, pixels = render("author", "author-" + str(i), request)
        expected = {}
        if 100 <= ms < 600:
            expected[objects[0]] = f.exact(Fraction(ms - 100, 500))
        if 1000 <= ms < 1500:
            expected[objects[1]] = f.exact(1 - Fraction(ms - 1000, 500))
        visibility = {objects[0]: "hidden" if ms < 100 else "visible"}
        if 1499 <= ms < 2000:
            visibility[objects[1]] = "hidden"
        assert state.get("opacity", {}) == expected, (ms, state)
        assert state["visibility"] == visibility, (ms, state)
        for j, source_file in enumerate([source, wps]):
            b = dict(binding, revision=source_file["sha256"])
            q = {
                "page": {
                    "profile": "drawingml-resource-page-q32-v1-draft",
                    "page": {
                        "expectedSourceSha256": source_file["sha256"],
                        "slide": "/ppt/slides/slide1.xml",
                        "profile": "drawingml-static-solid-page-v1-draft",
                        "colorContext": {"systemColors": {}, "placeholder": None},
                        "viewport": viewport,
                    },
                    "imageSource": "embeddedSnapshot",
                    "sampling": "nearest",
                    "fonts": None,
                },
                "sample": {"binding": b, "at": f.time(ms, 1000), "history": history(b)},
            }
            native, native_pixels = render("source", f"source-{j}-{i}", q, source_file)
            mapping = {objects[k]: "sp." + str(k + 2) for k in range(2)}
            assert native.get("opacity", {}) == {
                mapping[k]: v for k, v in expected.items()
            }, (ms, native)
            assert native["visibility"] == {
                mapping[k]: v for k, v in visibility.items()
            }, (ms, native)
            if j == 0:
                assert native_pixels == pixels, ms
        # Independent interior pixel oracle for opaque rectangles over white.
        for k in range(2):
            o = doc["objects"][objects[k]]
            t = o["transform"]
            color = o["appearance"]["fill"]["value"]["color"]["rgba"]
            x = (int(t["origin"]["x"]) + int(t["size"]["width"]) // 2) // 10000
            y = (int(t["origin"]["y"]) + int(t["size"]["height"]) // 2) // 10000
            p = (
                Fraction(
                    int(expected[objects[k]]["numerator"]),
                    int(expected[objects[k]]["denominator"]),
                )
                if objects[k] in expected
                else Fraction(1)
            )
            alpha16 = (p.numerator * 131070 + p.denominator) // (2 * p.denominator)
            alpha8 = (255 * alpha16 + 32767) // 65535
            rgb = [
                (color[c] * alpha16 + 32767) // 65535 + 255 - alpha8
                for c in ["red", "green", "blue"]
            ]
            if visibility.get(objects[k]) == "hidden":
                rgb = [255] * 3
            actual = list(pixels[(y * 800 + x) * 4 : (y * 800 + x) * 4 + 4])
            assert actual == rgb + [255], (ms, k, actual, rgb)
        checks.append(
            {"at": f.time(ms, 1000), "opacity": expected, "visibility": visibility}
        )

    # Existing owned source inputs contain real text outlines and embedded images.
    resources = json.loads(a.resources.read_bytes())
    for name in ["image-text", "group-image"]:
        case = next(c for c in resources["cases"] if c["name"] == name + "-0")
        raw = f.load(case["source"])
        fonts = case["fonts"]
        f.load(fonts)
        inputs[case["source"]["path"]] = case["source"]
        inputs[fonts["path"]] = fonts
        q = json.loads(f.load(case["request"]))
        with zipfile.ZipFile(io.BytesIO(raw)) as z:
            parts = {p: z.read(p) for p in z.namelist()}
        part = q["page"]["page"]["slide"].lstrip("/")
        tree = X.fromstring(parts[part])
        for old in tree.findall("p:timing", f.NS):
            tree.remove(old)
        targets = []
        for obj in tree.find("p:cSld/p:spTree", f.NS):
            c = obj.find(".//p:cNvPr", f.NS)
            if c is not None and obj.tag.split("}")[-1] in ["sp", "pic", "grpSp"]:
                targets.append(c.get("id"))
        timing = f.sub(tree, "timing")
        lst = f.sub(timing, "tnLst")
        root = f.sub(
            f.sub(lst, "par"),
            "cTn",
            id=1,
            dur="indefinite",
            restart="never",
            nodeType="tmRoot",
        )
        children = f.sub(root, "childTnLst")
        for i, target in enumerate(targets):
            effect = f.sub(children, "animEffect", filter="fade", transition="in")
            behavior = f.sub(effect, "cBhvr")
            f.sub(behavior, "cTn", id=i + 2, dur=1000)
            f.sub(f.sub(behavior, "tgtEl"), "spTgt", spid=target)
        parts[part] = X.tostring(tree, encoding="utf-8", xml_declaration=True)
        path = out / (name + ".pptx")
        with zipfile.ZipFile(path, "x", zipfile.ZIP_DEFLATED) as z:
            for p, b in sorted(parts.items()):
                # Own fixture bytes must not depend on the wall clock: source
                # identity also binds the evaluated playback metadata.
                info = zipfile.ZipInfo(p, date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, b)
        src = f.entry(path)
        validate(src)
        q["page"]["page"]["expectedSourceSha256"] = src["sha256"]
        q["sample"]["binding"]["revision"] = src["sha256"]
        q["sample"]["history"] = None
        for ms in [0, 250, 500, 999, 1000, 500]:
            request = copy.deepcopy(q)
            request["sample"]["at"] = f.time(ms, 1000)
            state, _ = render(
                "source", name + "-" + str(len(records["source"])), request, src, fonts
            )
            assert state.get("opacity", {}) == (
                {("sp." + target): f.exact(Fraction(ms, 1000)) for target in targets}
                if ms < 1000
                else {}
            )
    for kind, cases in records.items():
        f.put(out / (kind + ".json"), {"cases": cases})
    report = {
        "status": "passed",
        "inputs": inputs,
        "checks": checks,
        "xsd": validations,
        "frames": {k: len(v) for k, v in records.items()},
    }
    f.put(out / "report.json", report)
    print(
        json.dumps(
            {
                "status": "passed",
                "frames": report["frames"],
                "xsdParts": len(validations),
            }
        )
    )


if __name__ == "__main__":
    main()
