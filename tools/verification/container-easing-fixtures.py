"""Exact container-clock samples against independently re-timed flat controls.

Inputs are owned motion fixtures. Expected time is calculated with Fraction,
outside the kernel, and the entire raster is compared with the unfiltered graph.
This is computation evidence, not a WPS/Office or full-PPT acceptance verdict.
"""
import argparse
import copy
import importlib.util
import io
import json
from fractions import Fraction
from pathlib import Path
import zipfile
import xml.etree.ElementTree as X
from lxml import etree

spec = importlib.util.spec_from_file_location("motion", Path(__file__).with_name("motion-playback-fixtures.py"))
motion = importlib.util.module_from_spec(spec)
spec.loader.exec_module(motion)
f = motion.f


def filtered_time(at, nested):
    p = min(Fraction(1), at / 2)
    if nested:
        p *= p
    return 2 * (2 * p * p if p <= Fraction(1, 2) else 1 - 2 * (1 - p) ** 2)


def transform(accel, decel):
    return dict(speedMilliPercent=100000, autoReverse=False,
                accelerationMilliPercent=accel, decelerationMilliPercent=decel)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["output", "worker", "cli", "corpus", "schema"]:
        parser.add_argument("--" + name, required=True, type=Path)
    a = parser.parse_args()
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {}

    def read(record):
        data = f.load(record)
        inputs[record["path"]] = record
        return data

    for p in [a.worker, a.cli, a.schema, Path(__file__), Path(motion.__file__), Path(f.__file__)]:
        read(f.entry(p))
    prior_author = json.loads(read(f.entry(a.corpus / "author.json")))
    prior_source = json.loads(read(f.entry(a.corpus / "source.json")))
    cases = {"author": [], "source": []}
    controls, schemas = [], []
    schema = etree.XMLSchema(etree.parse(str(a.schema)))
    times = [Fraction(0), Fraction(1, 7), Fraction(1, 2), Fraction(1), Fraction(3, 2), Fraction(2), Fraction(3), Fraction(1, 7)]

    def render(kind, name, q, source=None, fonts=None):
        response, metadata, pixels = f.native(a.worker, q, read(source) if source else None, read(fonts) if fonts else None)
        c = dict(name=name, request=f.put(out / (name + ".request.json"), q),
                 response=f.put(out / (name + ".response.json"), metadata),
                 pixels=f.put(out / (name + ".rgba"), pixels))
        if source:
            c.update(source=source, fonts=fonts)
        cases[kind].append(c)
        return pixels

    author = json.loads(read(prior_author["cases"][0]["request"]))
    slide = author["playback"]["slide"]
    flat_doc = copy.deepcopy(author["playback"]["snapshot"]["document"])
    # The child's two iterations must consume container time, which differs
    # from easing each one-second iteration independently.
    flat_doc["timelines"][slide]["nodes"][1].update(duration=f.time(1, 1), repeatMilli=2000)
    flat = motion.initialize(a.cli, flat_doc)
    for nested in [False, True]:
        doc = copy.deepcopy(flat_doc)
        timeline = doc["timelines"][slide]
        container = dict(id="ease", kind="parallel", start=dict(kind="at", offset=f.time(0, 1)),
                         duration=dict(kind="automatic"), fill="hold",
                         children=[n["id"] for n in timeline["nodes"]], timeTransform=transform(50000, 50000))
        timeline.update(format="musteroffice.timeline/0.2-draft", tree=dict(roots=["ease"], containers=[container]))
        if nested:
            outer = copy.deepcopy(container)
            outer.update(id="outer", children=["ease"], timeTransform=transform(100000, 0))
            timeline["tree"].update(roots=["outer"], containers=[container, outer])
        snapshot = motion.initialize(a.cli, doc)
        for i, at in enumerate(times):
            q = copy.deepcopy(author)
            q["playback"].update(snapshot=snapshot, at=f.time(at.numerator, at.denominator))
            q["playback"]["binding"]["revision"] = snapshot["revision"]
            name = f"author-{'nested' if nested else 'single'}-{i}"
            pixels = render("author", name, q)
            expected_time = filtered_time(at, nested)
            control = copy.deepcopy(q)
            control["playback"].update(snapshot=flat, at=f.time(expected_time.numerator, expected_time.denominator))
            control["playback"]["binding"]["revision"] = flat["revision"]
            _, _, expected = f.native(a.worker, control)
            assert pixels == expected, name
            controls.append(dict(name=name, request=f.put(out / (name + ".control.json"), control), pixelSha256=f.sha(expected)))

    for name in ["image-text", "group-image", "circle"]:
        prior = next(c for c in prior_source["cases"] if c["name"] == name + "-0")
        original = read(prior["source"])
        with zipfile.ZipFile(io.BytesIO(original)) as z:
            parts = {p: z.read(p) for p in z.namelist()}
        baseline = json.loads(read(prior["request"]))
        part = baseline["page"]["page"]["slide"].lstrip("/")
        tree = X.fromstring(parts[part])
        listing = tree.find("p:timing/p:tnLst/p:par/p:cTn/p:childTnLst", f.NS)
        leaves = list(listing)
        listing.clear()
        # Separate noncommuting filters ensure preservation of cascade order.
        outer = f.sub(f.sub(listing, "par"), "cTn", id=10001, fill="hold", accel=100000)
        inner = f.sub(f.sub(f.sub(outer, "childTnLst"), "par"), "cTn", id=10002, fill="hold", accel=50000, decel=50000)
        inner_list = f.sub(inner, "childTnLst")
        inner_list.extend(leaves)
        parts[part] = X.tostring(tree)
        schema.assertValid(etree.fromstring(parts[part]))
        source = f.put(out / (name + ".pptx"), motion.archive(parts))
        schemas.append(dict(source=source, part=part))
        for i, at in enumerate(times):
            q = copy.deepcopy(baseline)
            q["page"]["page"]["expectedSourceSha256"] = source["sha256"]
            q["sample"]["binding"]["revision"] = source["sha256"]
            q["sample"]["at"] = f.time(at.numerator, at.denominator)
            case = name + "-" + str(i)
            pixels = render("source", case, q, source, prior["fonts"])
            expected_time = filtered_time(at, True)
            control = copy.deepcopy(baseline)
            control["sample"]["at"] = f.time(expected_time.numerator, expected_time.denominator)
            _, _, expected = f.native(a.worker, control, original, read(prior["fonts"]))
            assert pixels == expected, case
            controls.append(dict(name=case, source=prior["source"], request=f.put(out / (case + ".control.json"), control), pixelSha256=f.sha(expected)))
    for kind, records in cases.items():
        f.put(out / (kind + ".json"), dict(format="musteroffice.container-easing-native/1", cases=records))
    for record in inputs.values():
        f.load(record)
    report = dict(status="passed", inputs=inputs, nativeFrames=sum(map(len, cases.values())), independentClockControls=controls, xsdSlides=schemas,
                  scope="Coincident finite container easing; nonlinear deadline projection and Office/WPS full compatibility remain open.")
    f.put(out / "report.json", report)
    print(json.dumps(dict(status="passed", frames=report["nativeFrames"], controls=len(controls))))


if __name__ == "__main__":
    main()
