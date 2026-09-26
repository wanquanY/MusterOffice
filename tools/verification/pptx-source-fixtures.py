"""Independent Python ZIP/XML producer for native-source read/edit admission.

All content starts from our owned authored fixture. Extensions, signatures and
macro payloads here are synthetic preservation probes, not executable content or
valid cryptographic signatures. No user documents are collected.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET
import zipfile

P = "http://schemas.openxmlformats.org/presentationml/2006/main"
A = "http://schemas.openxmlformats.org/drawingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
C = "http://schemas.openxmlformats.org/package/2006/content-types"
REL = "http://schemas.openxmlformats.org/package/2006/relationships"
MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
NS = {"p": P, "a": A, "r": R}
for prefix, uri in {**NS, "mc": MC}.items():
    ET.register_namespace(prefix, uri)
SLIDE = "ppt/slides/slide1.xml"
MAIN = "ppt/presentation.xml"


def xml(data):
    return ET.fromstring(data)


def encode(root):
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.source) as z:
        base = {name: z.read(name) for name in z.namelist()}
    cases = []

    def emit(name, parts=None, **expect):
        path = args.directory / f"{name}.pptx"
        if parts is None:
            path.write_bytes(args.source.read_bytes())
        else:
            with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
                for key, value in parts.items():
                    info = zipfile.ZipInfo(key, (2026, 1, 1, 0, 0, 0))
                    info.compress_type = zipfile.ZIP_DEFLATED
                    z.writestr(info, value)
        cases.append({"name": name, "path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "expect": expect})

    emit("authored", edit="success", objects=15)
    parts = copy.deepcopy(base)
    text = parts[SLIDE].decode().replace("<p:cSld", '<!--same-part-comment--><p:cSld xmlns:q="urn:owned" q:flag="verbatim"')
    text = text.replace("</p:sld>", '<p:extLst><p:ext uri="urn:owned"><q:data xmlns:q="urn:owned">opaque &amp; payload</q:data></p:ext></p:extLst></p:sld>')
    parts[SLIDE] = text.encode()
    ct = xml(parts["[Content_Types].xml"])
    ET.SubElement(ct, f"{{{C}}}Override", PartName="/custom/opaque.bin", ContentType="application/octet-stream")
    parts["[Content_Types].xml"] = encode(ct)
    parts["custom/opaque.bin"] = bytes(range(256)) * 4
    emit("unknown-content", parts, edit="success", objects=15)
    utf = copy.deepcopy(parts)
    utf[SLIDE] = b"\xff\xfe" + text.replace('encoding="UTF-8"', 'encoding="UTF-16"').replace('encoding="utf-8"', 'encoding="UTF-16"').encode("utf-16-le")
    emit("utf16", utf, edit="success", objects=15)

    parts = copy.deepcopy(base)
    slide = xml(parts[SLIDE])
    identities = slide.findall("p:cSld/p:spTree/p:sp/p:nvSpPr/p:cNvPr", NS)
    identities[1].set("id", identities[0].get("id"))
    parts[SLIDE] = encode(slide)
    emit("duplicate-object-id", parts, error="INPUT_INVALID")

    for name, mutate in [
        ("duplicate-slide-id", lambda e: e.findall("p:sldIdLst/p:sldId", NS)[1].set("id", "256")),
        ("missing-slide-relationship", lambda e: e.find("p:sldIdLst/p:sldId", NS).set(f"{{{R}}}id", "absent")),
    ]:
        parts = copy.deepcopy(base)
        element = xml(parts[MAIN]); mutate(element); parts[MAIN] = encode(element)
        emit(name, parts, error="INPUT_INVALID")

    for name, prefix in [("main-must-understand", "p"), ("main-unknown-must-understand", "u")]:
        parts = copy.deepcopy(base)
        element = xml(parts[MAIN]); element.set(f"{{{MC}}}MustUnderstand", prefix)
        element.set("xmlns:u", "urn:owned:unsupported")
        parts[MAIN] = encode(element)
        emit(name, parts, **({"edit": "success", "objects": 15} if prefix == "p" else {"error": "MAPPING_NOT_IMPLEMENTED"}))

    for name in ["timing", "alternate-content", "inactive-timing", "invalid-alternate-content"]:
        parts = copy.deepcopy(base)
        element = xml(parts[SLIDE])
        if name == "timing":
            ET.SubElement(element, f"{{{P}}}timing")
        else:
            alternate = ET.SubElement(element, f"{{{MC}}}AlternateContent", {"xmlns:u": "urn:owned:unsupported"})
            if name != "invalid-alternate-content":
                choice = ET.SubElement(alternate, f"{{{MC}}}Choice", Requires="u")
                ET.SubElement(choice, f"{{{P}}}" + ("timing" if name == "inactive-timing" else "transition"))
            fallback = ET.SubElement(alternate, f"{{{MC}}}Fallback")
            ET.SubElement(fallback, f"{{{P}}}transition", spd="slow")
        parts[SLIDE] = encode(element)
        if name == "invalid-alternate-content":
            emit(name, parts, error="INPUT_INVALID")
        else:
            editable = name == "alternate-content"
            emit(name, parts, edit="success" if editable else "MAPPING_NOT_IMPLEMENTED", objects=15, editable=editable)

    for name in ["selected-shape", "fallback-shape", "process-content", "ignored-subtree", "structured-leaf"]:
        parts = copy.deepcopy(base)
        element = xml(parts[SLIDE]); element.set("xmlns:u", "urn:owned:unsupported")
        tree = element.find("p:cSld/p:spTree", NS)
        shape = tree.find("p:sp", NS); position = list(tree).index(shape)
        if name.endswith("shape"):
            tree.remove(shape)
            alternate = ET.Element(f"{{{MC}}}AlternateContent")
            choice = ET.SubElement(alternate, f"{{{MC}}}Choice", Requires="p" if name == "selected-shape" else "u")
            choice.append(copy.deepcopy(shape))
            fallback = ET.SubElement(alternate, f"{{{MC}}}Fallback"); fallback.append(shape)
            tree.insert(position, alternate)
        else:
            element.set(f"{{{MC}}}Ignorable", "u")
            if name == "process-content":
                element.set(f"{{{MC}}}ProcessContent", "u:wrap")
                tree.remove(shape); wrapper = ET.Element("u:wrap"); wrapper.append(shape); tree.insert(position, wrapper)
            elif name == "ignored-subtree":
                ignored = ET.Element("u:ignored"); ignored.append(copy.deepcopy(shape)); tree.insert(position, ignored)
                ignored.find("p:sp/p:txBody/a:p/a:r/a:t", NS).text = "忽略节点内的原生对象"
                shape.set("u:flag", "preserve")
            else:
                leaf = shape.find("p:txBody/a:p/a:r/a:t", NS)
                ET.SubElement(leaf, "u:payload").text = "retained"
        parts[SLIDE] = encode(element)
        editable = name in ["process-content", "ignored-subtree"]
        emit(name, parts, edit="success" if editable else "MAPPING_NOT_IMPLEMENTED", objects=15, editable=editable,
             constraint=None if editable else ("structuredLeaf" if name == "structured-leaf" else "compatibilityBranch"),
             outsideEdit=name.endswith("shape"))

    parts = copy.deepcopy(base)
    alternate = ET.Element(f"{{{MC}}}AlternateContent")
    ET.SubElement(alternate, f"{{{MC}}}Choice", Requires="p").append(xml(parts[MAIN]))
    parts[MAIN] = encode(alternate)
    emit("main-alternate-root", parts, edit="success", objects=15)

    parts = copy.deepcopy(base)
    element = xml(parts[SLIDE])
    run = element.find("p:cSld/p:spTree/p:sp/p:txBody/a:p/a:r", NS)
    run.tag = f"{{{A}}}fld"; run.set("id", "{00000000-0000-0000-0000-000000000001}"); run.set("type", "slidenum")
    parts[SLIDE] = encode(element)
    emit("field", parts, edit="SOURCE_CONFLICT", objects=15, editable=False)

    parts = copy.deepcopy(base)
    ct = xml(parts["[Content_Types].xml"])
    ET.SubElement(ct, f"{{{C}}}Override", PartName="/_xmlsignatures/origin.sigs", ContentType="application/vnd.openxmlformats-package.digital-signature-origin")
    parts["[Content_Types].xml"] = encode(ct); parts["_xmlsignatures/origin.sigs"] = b""
    emit("signature-origin", parts, edit="PRESERVATION_CONFLICT", objects=15, signature=True)

    parts = copy.deepcopy(base)
    ct = xml(parts["[Content_Types].xml"])
    for child in ct:
        if child.get("PartName") == "/" + MAIN:
            child.set("ContentType", "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml")
    ET.SubElement(ct, f"{{{C}}}Override", PartName="/ppt/vbaProject.bin", ContentType="application/vnd.ms-office.vbaProject")
    parts["[Content_Types].xml"] = encode(ct); parts["ppt/vbaProject.bin"] = b"owned synthetic opaque bytes; not a VBA program"
    relationships = xml(parts["ppt/_rels/presentation.xml.rels"])
    ET.SubElement(relationships, f"{{{REL}}}Relationship", Id="rMacro", Type="http://schemas.microsoft.com/office/2006/relationships/vbaProject", Target="vbaProject.bin")
    parts["ppt/_rels/presentation.xml.rels"] = encode(relationships)
    emit("macro-preservation", parts, edit="success", objects=15)
    manifest = {"format": "musteroffice.pptx-source-fixtures/1", "scope": "owned synthetic source projection/preservation probes only", "cases": cases}
    (args.directory / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"cases": len(cases), "manifest": str(args.directory / "manifest.json")}))


if __name__ == "__main__":
    main()
