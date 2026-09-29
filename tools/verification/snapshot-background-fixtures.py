"""Owned gradient-background windows under single and nested native Fade.

Compare every native pixel with two static controls and independent integer
composition. Preserve exact requests for a separate public WASM Worker run.
"""

import argparse
import copy
import importlib.util
import json
import zipfile
from fractions import Fraction
from pathlib import Path
from xml.etree import ElementTree as X

from lxml import etree

spec = importlib.util.spec_from_file_location(
    "f", Path(__file__).with_name("scale-playback-fixtures.py")
)
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ["output", "worker", "base", "request", "schema"]:
        p.add_argument("--" + name, required=True, type=Path)
    a = p.parse_args()
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = [
        f.entry(v) for v in [a.worker, a.base, a.request, a.schema, Path(__file__)]
    ]
    schema = etree.XMLSchema(etree.parse(str(a.schema)))
    with zipfile.ZipFile(a.base) as z:
        parts = {n: z.read(n) for n in z.namelist()}
    source = X.fromstring(parts["ppt/slides/slide1.xml"])
    for old in source.findall("p:timing", f.NS):
        source.remove(old)
    content = source.find("p:cSld", f.NS)
    for old in content.findall("p:bg", f.NS):
        content.remove(old)
    background = X.fromstring(
        f'<p:bg xmlns:p="{f.P}" xmlns:a="{f.A}"><p:bgPr><a:gradFill rotWithShape="0">'
        '<a:gsLst><a:gs pos="0"><a:srgbClr val="17496B"/></a:gs>'
        '<a:gs pos="100000"><a:srgbClr val="D9F3A7"/></a:gs></a:gsLst>'
        '<a:lin ang="0" scaled="1"/></a:gradFill></p:bgPr></p:bg>'
    )
    content.insert(0, background)
    tree = content.find("p:spTree", f.NS)
    template = copy.deepcopy(tree.find("p:sp", f.NS))
    for obj in list(tree):
        if obj.tag not in [f"{{{f.P}}}nvGrpSpPr", f"{{{f.P}}}grpSpPr"]:
            tree.remove(obj)

    def shape(identity, x, y, width, height, window=False):
        obj = copy.deepcopy(template)
        obj.find("p:nvSpPr/p:cNvPr", f.NS).set("id", str(identity))
        props = obj.find("p:spPr", f.NS)
        for child in list(props):
            props.remove(child)
        props.append(
            X.fromstring(
                f'<a:xfrm xmlns:a="{f.A}"><a:off x="{x}" y="{y}"/>'
                f'<a:ext cx="{width}" cy="{height}"/></a:xfrm>'
            )
        )
        props.append(
            X.fromstring(
                f'<a:prstGeom xmlns:a="{f.A}" prst="rect"><a:avLst/></a:prstGeom>'
            )
        )
        props.append(
            X.fromstring(
                f'<a:noFill xmlns:a="{f.A}"/>'
                if window
                else f'<a:solidFill xmlns:a="{f.A}"><a:srgbClr val="F12B19"/></a:solidFill>'
            )
        )
        props.append(X.fromstring(f'<a:ln xmlns:a="{f.A}"><a:noFill/></a:ln>'))
        for text in obj.findall("p:txBody", f.NS):
            obj.remove(text)
        if window:
            obj.set("useBgFill", "1")
        return obj

    backdrop = shape(43, 1000000, 500000, 4000000, 3000000)
    window = shape(42, 2000000, 1000000, 2000000, 1000000, True)
    group = X.fromstring(
        f'<p:grpSp xmlns:p="{f.P}" xmlns:a="{f.A}"><p:nvGrpSpPr>'
        '<p:cNvPr id="90" name="owned group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>'
        '<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="8000000" cy="4500000"/>'
        '<a:chOff x="0" y="0"/><a:chExt cx="8000000" cy="4500000"/></a:xfrm></p:grpSpPr></p:grpSp>'
    )
    group.append(copy.deepcopy(window))
    records, xml_checks, checks = [], [], []
    empty = f.put(out / "empty.bin", b"")

    def pack(name, earlier, present, nested, animated):
        page = copy.deepcopy(source)
        objects = page.find("p:cSld/p:spTree", f.NS)
        if earlier:
            objects.append(copy.deepcopy(backdrop))
        if present:
            objects.append(copy.deepcopy(group if nested else window))
        if animated:
            timing = f.sub(page, "timing")
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
            for i, target in enumerate([90, 42] if nested else [42]):
                effect = f.sub(children, "animEffect", filter="fade", transition="in")
                behavior = f.sub(effect, "cBhvr")
                f.sub(behavior, "cTn", id=i + 2, dur=1000)
                f.sub(f.sub(behavior, "tgtEl"), "spTgt", spid=target)
        xml = X.tostring(page, encoding="utf-8", xml_declaration=True)
        schema.assertValid(etree.fromstring(xml))
        xml_checks.append(name)
        path = out / (name + ".pptx")
        with zipfile.ZipFile(path, "x") as z:
            for part, b in sorted(parts.items()):
                info = zipfile.ZipInfo(part, date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, xml if part == "ppt/slides/slide1.xml" else b)
        return f.entry(path)

    def render(name, source, ms):
        q = json.loads(a.request.read_bytes())
        q["page"]["page"]["expectedSourceSha256"] = source["sha256"]
        q["sample"]["binding"]["revision"] = source["sha256"]
        q["sample"]["history"] = None
        q["sample"]["at"] = f.time(ms, 1000)
        _, metadata, pixels = f.native(a.worker, q, f.load(source), b"")
        records.append(
            {
                "name": name,
                "request": f.put(out / (name + ".request.json"), q),
                "response": f.put(out / (name + ".response.json"), metadata),
                "pixels": f.put(out / (name + ".rgba"), pixels),
                "source": source,
                "fonts": empty,
            }
        )
        return pixels

    for name, earlier, nested in [
        ("first", False, False),
        ("later", True, False),
        ("nested", True, True),
    ]:
        before = pack(name + "-without", earlier, False, nested, False)
        after = pack(name + "-static", earlier, True, nested, False)
        dynamic = pack(name + "-fade", earlier, True, nested, True)
        baseline = render(name + "-without", before, 0)
        final = render(name + "-static", after, 0)
        for i, ms in enumerate([0, 250, 500, 999, 1000, 500]):
            actual = render(f"{name}-{i}", dynamic, ms)
            value = Fraction(min(ms, 1000), 1000)
            alpha = (value.numerator * 131070 + value.denominator) // (
                2 * value.denominator
            )
            expected = bytearray(baseline)
            # Integer-aligned 200x100 window: all pixels have full path coverage.
            for y in range(100, 200):
                for x in range(200, 400):
                    offset = (y * 800 + x) * 4
                    rgba = list(final[offset : offset + 4])
                    for _ in range(2 if nested else 1):
                        rgba = [(c * alpha + 32767) // 65535 for c in rgba]
                    for k in range(4):
                        expected[offset + k] = (
                            rgba[k]
                            + (baseline[offset + k] * (255 - rgba[3]) + 127) // 255
                        )
            assert actual == expected, (name, ms, "pixel mismatch")
            checks.append({"case": name, "at": f.time(ms, 1000), "nested": nested})
    f.put(out / "source.json", {"cases": records})
    f.put(out / "author.json", {"cases": []})
    f.put(
        out / "report.json",
        {
            "status": "passed",
            "inputs": inputs,
            "frames": len(records),
            "independentChecks": checks,
            "xsdSlides": xml_checks,
        },
    )
    print(
        json.dumps(
            {"status": "passed", "frames": len(records), "xsdSlides": len(xml_checks)}
        )
    )


if __name__ == "__main__":
    main()
