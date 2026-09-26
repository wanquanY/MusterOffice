"""Owned source packages exercising actual slide/layout/master inheritance.

Appends to the source corpus without borrowing a vendor template. This producer
uses Python ZIP/XML, independently of the Rust reader and package rewriter.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET
import zipfile
from mce_reference import P, A, R, MC

NS = {"p": P, "a": A}
C = "http://schemas.openxmlformats.org/package/2006/content-types"
REL = "http://schemas.openxmlformats.org/package/2006/relationships"
SLIDE = "ppt/slides/slide1.xml"
LAYOUT = "ppt/slideLayouts/slideLayout2.xml"
MASTER = "ppt/slideMasters/slideMaster2.xml"
for prefix, ns in {**NS, "r": R, "mc": MC}.items():
    ET.register_namespace(prefix, ns)


def encoded(root):
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def shape(root, name):
    return next(n for n in root.findall("p:cSld/p:spTree/p:sp", NS)
                if n.find("p:nvSpPr/p:cNvPr", NS).get("name") == name)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source_manifest", type=Path)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(args.source_manifest.read_text())
    with zipfile.ZipFile(manifest["cases"][0]["path"]) as z:
        base = {n: z.read(n) for n in z.namelist()}
    hierarchy = copy.deepcopy(base)
    for part, name, attributes, origin, size in [
        (MASTER, "footer:master", {"type": "body", "idx": "91"}, (111, 222), (333, 444)),
        (LAYOUT, "rule:layout", {"idx": "7"}, None, (555, 666)),
        (SLIDE, "title:1", {"type": "body", "idx": "7"}, None, None),
    ]:
        root = ET.fromstring(hierarchy[part]); obj = shape(root, name)
        ET.SubElement(obj.find("p:nvSpPr/p:nvPr", NS), f"{{{P}}}ph", attributes)
        props = obj.find("p:spPr", NS); props.remove(props.find("a:xfrm", NS))
        if origin is not None or size is not None:
            transform = ET.Element(f"{{{A}}}xfrm"); props.insert(0, transform)
            if origin is not None:
                ET.SubElement(transform, f"{{{A}}}off", x=str(origin[0]), y=str(origin[1]))
            if size is not None:
                ET.SubElement(transform, f"{{{A}}}ext", cx=str(size[0]), cy=str(size[1]))
        hierarchy[part] = encoded(root)

    def emit(name, parts, **expect):
        path = args.directory / f"{name}.pptx"
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
            for key, value in parts.items():
                info = zipfile.ZipInfo(key, (2026, 1, 1, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, value)
        manifest["cases"].append({"name": name, "path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "expect": expect})

    emit("inheritance-chain", hierarchy, edit="success", objects=15, inheritance="matched")
    visible = copy.deepcopy(hierarchy)
    for part, name, origin, size in [(MASTER, "footer:master", (914400, 914400), (7000000, 1200000)),
                                    (LAYOUT, "rule:layout", None, (5486400, 914400))]:
        root = ET.fromstring(visible[part]); transform = shape(root, name).find("p:spPr/a:xfrm", NS)
        if origin is not None:
            transform.find("a:off", NS).attrib.update(x=str(origin[0]), y=str(origin[1]))
        transform.find("a:ext", NS).attrib.update(cx=str(size[0]), cy=str(size[1]))
        visible[part] = encoded(root)
    emit("inheritance-visible", visible, edit="success", objects=15, inheritance="matched", originX="914400", width="5486400")
    for name in ["explicit-zero", "type-differs-index-matches", "unmatched", "detached", "unsupported-context", "invalid-placeholder"]:
        parts = copy.deepcopy(hierarchy); root = ET.fromstring(parts[SLIDE]); obj = shape(root, "title:1")
        ph = obj.find("p:nvSpPr/p:nvPr/p:ph", NS)
        if name == "explicit-zero":
            xfrm = ET.Element(f"{{{A}}}xfrm"); obj.find("p:spPr", NS).insert(0, xfrm)
            ET.SubElement(xfrm, f"{{{A}}}off", x="0", y="0"); ET.SubElement(xfrm, f"{{{A}}}ext", cx="0", cy="0")
        elif name == "type-differs-index-matches":
            ph.set("type", "title")
        elif name == "unmatched":
            ph.set("idx", "700")
        elif name == "detached":
            ph.set("idx", "4294967295")
        else:
            ph.set("type", "hdr" if name == "unsupported-context" else "invalid")
        parts[SLIDE] = encoded(root)
        if name == "invalid-placeholder":
            emit(name, parts, error="INPUT_INVALID")
        else:
            status = {"explicit-zero": "matched", "type-differs-index-matches": "matched", "unsupported-context": "unsupportedContext"}.get(name, name)
            emit(name, parts, edit="success", objects=15, inheritance=status, zero=name == "explicit-zero")

    for part, name in [(LAYOUT, "rule:layout"), (MASTER, "footer:master")]:
        parts = copy.deepcopy(hierarchy); root = ET.fromstring(parts[part]); duplicate = copy.deepcopy(shape(root, name))
        identity = duplicate.find("p:nvSpPr/p:cNvPr", NS); identity.set("id", "888"); identity.set("name", "ambiguous")
        root.find("p:cSld/p:spTree", NS).append(duplicate); parts[part] = encoded(root)
        emit("ambiguous-" + ("layout" if part == LAYOUT else "master"), parts, edit="success", objects=16,
             inheritance="ambiguous" if part == LAYOUT else "matched", ambiguousMaster=part == MASTER)

    parts = copy.deepcopy(hierarchy)
    for part, name, kind in [(LAYOUT, "rule:layout", "ctrTitle"), (MASTER, "footer:master", "title")]:
        root = ET.fromstring(parts[part]); shape(root, name).find("p:nvSpPr/p:nvPr/p:ph", NS).set("type", kind)
        parts[part] = encoded(root)
    emit("title-family", parts, edit="success", objects=15, inheritance="matched")

    parts = copy.deepcopy(hierarchy); root = ET.fromstring(parts[LAYOUT]); tree = root.find("p:cSld/p:spTree", NS)
    obj = shape(root, "rule:layout"); index = list(tree).index(obj); tree.remove(obj)
    alternate = ET.Element(f"{{{MC}}}AlternateContent"); ET.SubElement(alternate, f"{{{MC}}}Choice", Requires="p").append(obj)
    tree.insert(index, alternate); parts[LAYOUT] = encoded(root)
    emit("mce-placeholder-parent", parts, edit="success", objects=15, inheritance="matched")

    parts = copy.deepcopy(hierarchy)
    content_types = ET.fromstring(parts["[Content_Types].xml"])
    for parent, number in [(LAYOUT, 1), (SLIDE, 2)]:
        override = f"ppt/theme/override{number}.xml"
        parts[override] = encoded(ET.Element(f"{{{A}}}themeOverride"))
        ET.SubElement(content_types, f"{{{C}}}Override", PartName="/" + override, ContentType="application/vnd.openxmlformats-officedocument.themeOverride+xml")
        relpath = str(Path(parent).parent / "_rels" / (Path(parent).name + ".rels"))
        rels = ET.fromstring(parts[relpath])
        ET.SubElement(rels, f"{{{REL}}}Relationship", Id=f"override{number}", Type=R + "/themeOverride", Target=f"../theme/override{number}.xml")
        parts[relpath] = encoded(rels)
    parts["[Content_Types].xml"] = encoded(content_types)
    emit("theme-override-chain", parts, edit="success", objects=15, inheritance="matched", themeOverrides=2)
    wrong = copy.deepcopy(parts); content_types.find(f"{{{C}}}Override[@PartName='/ppt/theme/override1.xml']").set("ContentType", "application/xml")
    wrong["[Content_Types].xml"] = encoded(content_types)
    emit("theme-type-mismatch", wrong, error="INPUT_INVALID")

    parts = copy.deepcopy(hierarchy); relpath = "ppt/slides/_rels/slide1.xml.rels"; rels = ET.fromstring(parts[relpath])
    ET.SubElement(rels, f"{{{REL}}}Relationship", Id="second-layout", Type=R + "/slideLayout", Target="../slideLayouts/slideLayout1.xml")
    parts[relpath] = encoded(rels)
    emit("duplicate-layout-relationship", parts, error="INPUT_INVALID")
    (args.directory / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"cases": len(manifest["cases"]), "manifest": str(args.directory / "manifest.json")}))


if __name__ == "__main__":
    main()
