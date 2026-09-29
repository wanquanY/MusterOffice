"""Check owned native motion defaults against exact offsets and static geometry.

The inputs are the preserved WPS-authored line and the public-authoring two-group
control. This is a bounded computation check, not general Office acceptance.
"""
import argparse
import copy
from fractions import Fraction
import importlib.util
import io
import json
from pathlib import Path
import xml.etree.ElementTree as X
import zipfile
from lxml import etree

spec = importlib.util.spec_from_file_location("native", Path(__file__).with_name("scale-playback-fixtures.py"))
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)


def archive(parts):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, "w", compression=zipfile.ZIP_DEFLATED) as out:
        for name, data in parts.items():
            out.writestr(name, data)
    return stream.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ["output", "sources", "worker", "template", "schema"]:
        parser.add_argument("--" + key, type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {}

    def tracked(path):
        record = f.entry(path.resolve())
        inputs[record["path"]] = record
        return f.load(record)

    for path in [args.worker, Path(__file__), Path(f.__file__)]:
        tracked(path)
    for path in args.schema.parent.glob("*.xsd"):
        tracked(path)
    schema = etree.XMLSchema(etree.parse(str(args.schema)))
    template = json.loads(tracked(args.template))
    times = [0, 250, 750, 1000, 1500, 2500, 3000, 3250, 3500, 3750, 4000, 4500, 5000, 5500, 3000, 250]
    records, checks, controls, schema_parts = [], [], [], []
    fonts = f.put(out / "empty.bin", b"")
    for name in ["motion-defaults", "wps-authored-line"]:
        raw = tracked(args.sources / (name + ".pptx"))
        source = f.put(out / (name + ".pptx"), raw)
        with zipfile.ZipFile(io.BytesIO(raw)) as z:
            parts = {p: z.read(p) for p in z.namelist()}
        for part, data in parts.items():
            if not part.endswith(".xml"):
                continue
            root = etree.fromstring(data)
            if root.tag.startswith("{" + f.P + "}"):
                schema.assertValid(root)
                schema_parts.append(dict(source=source, part=part))
        slide = "ppt/slides/slide1.xml"
        tree = X.fromstring(parts[slide])
        if name == "wps-authored-line":
            paths = tree.findall(".//p:animMotion", f.NS)
            assert len(paths) == 1 and paths[0].get("path").strip() == "M 0 0 L 0.098984 0"
            assert paths[0].find("p:cBhvr", f.NS).get("additive") is None
        for timing in tree.findall("p:timing", f.NS):
            tree.remove(timing)
        size = X.fromstring(parts["ppt/presentation.xml"]).find("p:sldSz", f.NS)
        width = int(size.get("cx"))
        binding = dict(session=name, revision=source["sha256"], generation="1")
        q = copy.deepcopy(template)
        q["page"]["page"]["expectedSourceSha256"] = source["sha256"]
        q["sample"]["binding"] = binding
        q["sample"]["history"] = dict(binding=binding, through=f.time(10), events=[dict(
            at=f.time(3), generation="1", sequence=1,
            event=dict(kind="navigation", direction="next", target=None))])
        for index, ms in enumerate(times):
            at = Fraction(ms, 1000)
            q["sample"]["at"] = f.time(ms, 1000)
            response, metadata, pixels = f.native(args.worker, q, raw, b"")
            if name == "motion-defaults":
                offset = Fraction(3, 20) * min(at, 1) if at < 3 else Fraction(1, 10) * min(at - 3, 1)
                targets = list(range(2, 8))
            else:
                progress = min(max((at - 3) / 2, 0), 1)
                eased = 2 * progress * progress if progress <= Fraction(1, 2) else 1 - 2 * (1 - progress) ** 2
                offset = Fraction(12373, 125000) * eased
                targets = [2] if at >= 3 else []
            expected = {"sp." + str(target): dict(x=f.exact(offset), y=f.exact(Fraction(0))) for target in targets}
            state = response["info"]["playback"]["evaluated"]["state"]
            assert state.get("motion", {}) == expected, (name, ms, state.get("motion", {}), expected)
            stem = name + "-" + str(index)
            records.append(dict(name=stem, request=f.put(out / (stem + ".request.json"), q),
                                response=f.put(out / (stem + ".response.json"), metadata),
                                pixels=f.put(out / (stem + ".rgba"), pixels), source=source, fonts=fonts))
            checks.append(dict(name=stem, expectedMotion=expected))
            delta = offset * width
            if delta.denominator == 1:
                control = copy.deepcopy(tree)
                for obj in control.findall("p:cSld/p:spTree/p:sp", f.NS):
                    native_id = int(obj.find("p:nvSpPr/p:cNvPr", f.NS).get("id"))
                    if native_id in targets:
                        off = obj.find("p:spPr/a:xfrm/a:off", f.NS)
                        off.set("x", str(int(off.get("x")) + int(delta)))
                static_parts = dict(parts)
                static_parts[slide] = X.tostring(control)
                static = archive(static_parts)
                qc = copy.deepcopy(q["page"])
                qc["page"]["expectedSourceSha256"] = f.sha(static)
                _, _, expected_pixels = f.native(args.worker, qc, static, b"", "--pptx-resource-page")
                assert pixels == expected_pixels, stem + " static-position oracle"
                controls.append(dict(name=stem, source=f.put(out / (stem + ".control.pptx"), static),
                                     request=f.put(out / (stem + ".control.json"), qc), pixelSha256=f.sha(expected_pixels)))
    f.put(out / "source.json", dict(cases=records))
    f.put(out / "author.json", dict(cases=[]))
    for record in inputs.values():
        f.load(record)
    report = dict(status="passed", inputs=inputs, frames=len(records), checks=checks,
                  staticControls=controls, xsdParts=schema_parts)
    f.put(out / "report.json", report)
    print(json.dumps(dict(status="passed", frames=len(records), staticControls=len(controls), xsdParts=len(schema_parts))))


if __name__ == "__main__":
    main()
