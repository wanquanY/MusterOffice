"""Owned straight motion paths, exact rational samples and static-layout pixel oracles.

This verifies the declared computation profile, not Office/WPS playback fidelity.
Native source inputs keep images, text and group structure, without flattening.
"""
import argparse
import copy
import importlib.util
import io
import json
import subprocess
import zipfile
import xml.etree.ElementTree as X
from fractions import Fraction
from pathlib import Path
from lxml import etree

spec = importlib.util.spec_from_file_location("f", Path(__file__).with_name("scale-playback-fixtures.py"))
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)


def initialize(cli, document):
    result = subprocess.run([str(cli)], input=json.dumps(dict(operation="initialize", document=document)).encode(), capture_output=True, env={}, timeout=60)
    assert result.returncode == 0 and not result.stderr, result.stderr
    return json.loads(result.stdout)["snapshot"]


def archive(parts):
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w") as z:
        for part, data in parts.items():
            info = zipfile.ZipInfo(part, (2026, 9, 28, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
    return out.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ["output", "worker", "cli", "author", "source", "schema"]:
        parser.add_argument("--" + key, type=Path, required=True)
    a = parser.parse_args()
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): f.entry(p) for p in [a.worker, a.cli, a.author, a.source, a.schema, Path(__file__)]}
    records = {"author": [], "source": []}
    controls, schemas = [], []
    times = [(0, 1), (1, 7), (1, 2), (1, 1), (2, 1), (1, 7), (0, 1)]
    schema = etree.XMLSchema(etree.parse(str(a.schema)))

    def read(record):
        inputs[record["path"]] = record
        return f.load(record)

    def render(kind, name, q, source=None, fonts=None):
        response, metadata, pixels = f.native(a.worker, q, read(source) if source else None, read(fonts) if fonts else None)
        c = dict(name=name, request=f.put(out/(name+".request.json"), q), response=f.put(out/(name+".response.json"), metadata), pixels=f.put(out/(name+".rgba"), pixels))
        if source:
            c.update(source=source, fonts=fonts)
        records[kind].append(c)
        state = response["info"]["frame"]["state"] if kind == "author" else response["info"]["playback"]["evaluated"]["state"]
        assert state["profile"] == "musteroffice.motion-frame/0.1-draft"
        return c, state, pixels

    page = json.loads(a.author.read_bytes())
    original = copy.deepcopy(page["page"]["document"])
    doc = page["page"]["document"]
    node = copy.deepcopy(doc["timelines"][page["page"]["slide"]]["nodes"][0])
    node["id"] = "motion:group"
    node["effect"] = dict(kind="motionLine", target="group:1", **{"from": dict(x="0", y="0"), "to": dict(x="0.125", y="-0.125")})
    doc["timelines"][page["page"]["slide"]]["nodes"].append(node)
    snapshot = initialize(a.cli, doc)
    for i, (n, d) in enumerate(times):
        at = f.time(n, d)
        binding = dict(session="motion", revision=snapshot["revision"], generation="1")
        q = dict(playback=dict(snapshot=snapshot, slide=page["page"]["slide"], binding=binding, at=at, history=None), viewport=page["viewport"], defaults=page["defaults"])
        _, state, pixels = render("author", "author-"+str(i), q)
        offset = Fraction(n, 16*d)
        assert state["motion"] == {"group:1": dict(x=f.exact(offset), y=f.exact(-offset))}
        deltas = [offset*int(doc["pageSize"]["width"]), -offset*int(doc["pageSize"]["height"])]
        if all(v.denominator == 1 for v in deltas):
            control = copy.deepcopy(original)
            origin = control["objects"]["group:1"]["transform"]["origin"]
            for axis, delta in zip(["x", "y"], deltas):
                origin[axis] = str(int(origin[axis])+int(delta))
            qc = copy.deepcopy(q)
            qc["playback"]["snapshot"] = initialize(a.cli, control)
            qc["playback"]["binding"]["revision"] = qc["playback"]["snapshot"]["revision"]
            _, _, expected = f.native(a.worker, qc)
            assert pixels == expected, "author static-position oracle"
            controls.append(dict(name="author-"+str(i), request=f.put(out/("control-author-"+str(i)+".json"),qc), pixelSha256=f.sha(expected)))

    previous = json.loads(a.source.read_bytes())
    for name in ["image-text", "group-image", "circle"]:
        prior = next(c for c in previous["cases"] if c["name"] == name+"-0")
        with zipfile.ZipFile(io.BytesIO(read(prior["source"]))) as z:
            parts = {key: z.read(key) for key in z.namelist()}
        q = json.loads(read(prior["request"]))
        fonts = prior["fonts"]
        slide = q["page"]["page"]["slide"].lstrip("/")
        tree = X.fromstring(parts[slide])
        for old in tree.findall("p:timing", f.NS):
            tree.remove(old)
        static_tree = copy.deepcopy(tree)
        roots = [o for o in tree.find("p:cSld/p:spTree",f.NS) if o.tag in ["{"+f.P+"}"+s for s in ["sp","pic","grpSp"]]]
        ids = [int(o.find("./*/p:cNvPr",f.NS).get("id")) for o in roots]
        assert ids
        listing = f.sub(f.sub(f.sub(f.sub(f.sub(tree,"timing"),"tnLst"),"par"),"cTn",id=1,dur="indefinite",restart="never",nodeType="tmRoot"),"childTnLst")
        for i, target in enumerate(ids):
            anim = f.sub(listing,"animMotion",origin="layout",path="M 0 0 L 0.125 -0.125 E",pathEditMode="relative")
            behavior = f.sub(anim,"cBhvr",additive="repl",accumulate="none",xfrmType="pt")
            common = f.sub(behavior,"cTn",id=i+2,dur=2000,repeatCount=1000,restart="never",fill="freeze")
            f.sub(f.sub(common,"stCondLst"),"cond",delay=0)
            f.sub(f.sub(behavior,"tgtEl"),"spTgt",spid=target)
            names = f.sub(behavior,"attrNameLst")
            for axis in ["ppt_x","ppt_y"]: f.sub(names,"attrName").text=axis
        parts[slide] = X.tostring(tree)
        schema.assertValid(etree.fromstring(parts[slide]))
        source = f.put(out/(name+".pptx"),archive(parts))
        schemas.append(dict(source=source,part=slide))
        q["page"]["page"]["expectedSourceSha256"] = source["sha256"]
        q["sample"]["binding"]["revision"] = source["sha256"]
        size = X.fromstring(parts["ppt/presentation.xml"]).find("p:sldSz",f.NS)
        for i,(n,d) in enumerate(times):
            q["sample"]["at"] = f.time(n,d)
            _,state,pixels = render("source",name+"-"+str(i),q,source,fonts)
            offset = Fraction(n,16*d)
            assert state["motion"] == {"sp."+str(target):dict(x=f.exact(offset),y=f.exact(-offset)) for target in ids}
            deltas = [offset*int(size.get("cx")),-offset*int(size.get("cy"))]
            if all(v.denominator == 1 for v in deltas):
                control = copy.deepcopy(static_tree)
                for root in control.find("p:cSld/p:spTree",f.NS):
                    if root.tag not in ["{"+f.P+"}"+s for s in ["sp","pic","grpSp"]]: continue
                    off = root.find("./*/a:xfrm/a:off",f.NS)
                    assert off is not None
                    for axis,delta in zip(["x","y"],deltas): off.set(axis,str(int(off.get(axis))+int(delta)))
                control_parts=dict(parts);control_parts[slide]=X.tostring(control)
                raw=archive(control_parts); qc=copy.deepcopy(q["page"])
                qc["page"]["expectedSourceSha256"]=f.sha(raw)
                _,_,expected=f.native(a.worker,qc,raw,read(fonts),"--pptx-resource-page")
                assert pixels == expected, name+" static-position oracle"
                controls.append(dict(name=name+"-"+str(i),source=f.put(out/(name+"-control-"+str(i)+".pptx"),raw),request=f.put(out/(name+"-control-"+str(i)+".json"),qc),pixelSha256=f.sha(expected)))
    for kind,cases in records.items(): f.put(out/(kind+".json"),dict(format="musteroffice.motion-native/1",cases=cases))
    for record in inputs.values(): f.load(record)
    f.put(out/"report.json",dict(status="passed",inputs=inputs,nativeFrames=sum(map(len,records.values())),staticControls=controls,xsdSlides=schemas,scope="Straight layout-relative motion computation; Office/WPS calibration and full path coverage remain open."))
    print(json.dumps(dict(status="passed",frames=sum(map(len,records.values())),staticControls=len(controls))))


if __name__ == "__main__": main()
