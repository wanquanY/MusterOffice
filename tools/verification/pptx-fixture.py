"""Generate an owned, synthetic author document and a small PNG resource bundle.
This is test input authoring, not a second PPTX writer or a product template.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import zlib


def rgba(hex_value, alpha=255):
    return dict(zip(["red", "green", "blue", "alpha"], [int(hex_value[i:i+2], 16) for i in [0, 2, 4]] + [alpha]))


def explicit(value):
    return {"kind": "value", "value": value}


def solid(hex_value):
    return explicit({"kind": "solid", "color": {"kind": "srgb", "rgba": rgba(hex_value)}})


def png():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = []
    for y in range(48):
        row = bytearray(b"\0")
        for x in range(48):
            row.extend((27, 114, 232) if (x//8 + y//8) % 2 else (232, 241, 254))
        rows.append(row)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 48, 48, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(b"".join(rows))) + chunk(b"IEND", b"")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    objects = {}
    shape_ids = {"slide:1": [], "slide:2": []}

    def obj(id, parent, x, y, w, h, content, fill="FFFFFF", stroke=False):
        record = {"id": id, "parent": parent, "transform": {"origin": {"x": str(x), "y": str(y)}, "size": {"width": str(w), "height": str(h)}, "rotation": 0, "flipHorizontal": False, "flipVertical": False}, "appearance": {"fill": solid(fill), "stroke": explicit({"kind": "none"}) if not stroke else explicit({"kind": "solid", "color": {"kind": "srgb", "rgba": rgba("1B72E8")}, "width": "25400"})}, "accessibility": {"title": id, "description": "Owned synthetic native-object test", "decorative": False}, "content": content}
        objects[id] = record
        if parent["kind"] == "slide":
            shape_ids[parent["id"]].append(id)
        return record

    def shape(id, slide, x, y, w, h, fill="FFFFFF", geometry=None):
        return obj(id, {"kind": "slide", "id": slide}, x, y, w, h, {"kind": "shape", "geometry": geometry or {"kind": "rectangle"}, "text": None}, fill)

    def label(id, parent, x, y, w, h, lines, size=24, color="162238", fill="FFFFFF"):
        paragraphs = []
        for i, line in enumerate(lines):
            paragraphs.append({"id": f"paragraph:{id}:{i}", "style": {}, "defaultRunStyle": {}, "runs": [{"id": f"run:{id}:{i}", "style": {}, "content": {"kind": "text", "text": line}}]})
        body = {"paragraphs": paragraphs, "style": {"size": explicit(str(size * 12700)), "color": explicit({"kind": "srgb", "rgba": rgba(color)})}, "insets": {"left": "0", "top": "0", "right": "0", "bottom": "0"}, "wrap": True, "overflow": "report"}
        return obj(id, parent, x, y, w, h, {"kind": "shape", "geometry": {"kind": "rectangle"}, "text": body}, fill)

    slide1 = {"kind": "slide", "id": "slide:1"}
    slide2 = {"kind": "slide", "id": "slide:2"}
    label("title:1", slide1, 600000, 500000, 10900000, 700000, ["Native objects. Editable content."], 32)
    label("subtitle:1", slide1, 600000, 1350000, 10000000, 650000, ["MusterOffice · PresentationML export verification"], 18, "52647A")
    shape("round:1", "slide:1", 600000, 2500000, 2743200, 914400, "1B72E8", {"kind": "roundRectangle", "radius": "182880"})
    shape("ellipse:1", "slide:1", 4000000, 2450000, 1800000, 1800000, "26B99A", {"kind": "ellipse"})
    image = obj("picture:1", slide1, 7000000, 2450000, 1800000, 1800000, {"kind": "picture", "resource": "resource:checker", "crop": {"left": 0, "top": 0, "right": 0, "bottom": 0}})
    image["transform"]["rotation"] = 600000
    label("unicode:1", slide1, 600000, 4800000, 10500000, 800000, ["中文 / العربية / 🚀 / é", "Source text stays text; shapes stay native shapes."], 20)

    label("title:2", slide2, 600000, 500000, 10900000, 700000, ["Groups, paths and relationships"], 32)
    group = obj("group:2", slide2, 700000, 2100000, 4500000, 2200000, {"kind": "group", "children": ["child:2a", "child:2b"], "viewport": {"width": "4500000", "height": "2200000"}})
    group["appearance"] = {}
    group["transform"]["rotation"] = -300000
    parent = {"kind": "group", "id": "group:2"}
    label("child:2a", parent, 0, 0, 4000000, 650000, ["Editable group child"], 22, fill="E8F1FE")
    obj("child:2b", parent, 100000, 1000000, 3000000, 1000000, {"kind": "shape", "geometry": {"kind": "path", "viewport": {"width": "3000000", "height": "1000000"}, "commands": [{"kind": "move", "to": {"x": "0", "y": "1000000"}}, {"kind": "quadratic", "control": {"x": "1500000", "y": "0"}, "to": {"x": "3000000", "y": "1000000"}}, {"kind": "close"}]}, "text": None}, "26B99A")
    shape("target:2", "slide:2", 7200000, 2500000, 2100000, 1400000, "1B72E8")
    connector = obj("connector:2", slide2, 5200000, 3200000, 2000000, 0, {"kind": "connector", "start": {"kind": "attached", "object": "group:2", "site": 1}, "end": {"kind": "attached", "object": "target:2", "site": 1}}, stroke=True)
    # A supported native shape site; group connection-site semantics remain unimplemented.
    connector["content"]["start"] = {"kind": "free", "position": {"x": "5200000", "y": "3200000"}}
    connector["appearance"]["fill"] = explicit({"kind": "none"})
    label("detail:2", slide2, 600000, 5000000, 10500000, 800000, ["Two pages · one shared theme · native layout and master"], 20)
    label("footer:master", {"kind": "master", "id": "master:brand"}, 600000, 6350000, 11000000, 250000, ["SYNTHETIC VERIFICATION DOCUMENT"], 10, "52647A")
    obj("rule:layout", {"kind": "layout", "id": "layout:brand"}, 600000, 180000, 10900000, 50000, {"kind": "shape", "geometry": {"kind": "rectangle"}, "text": None}, "1B72E8")

    slots = ["dark1", "light1", "dark2", "light2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hyperlink", "followedHyperlink"]
    palette = ["162238", "FFFFFF", "52647A", "E8F1FE", "1B72E8", "26B99A", "F2AD3B", "8562D8", "E5768D", "5BA6B8", "0563C1", "954F72"]
    colors = {name: rgba(color) for name, color in zip(slots, palette)}
    data = png()
    document = {"format": "musteroffice.presentation/0.1-draft", "id": "document:native-fixture", "title": "MusterOffice native PPTX verification", "pageSize": {"width": "12192000", "height": "6858000"}, "slideOrder": ["slide:1", "slide:2"], "slides": {id: {"id": id, "name": f"Native page {i+1}", "layout": "layout:brand", "objects": shape_ids[id], "background": {"kind": "inherit"}, "hidden": False} for i, id in enumerate(shape_ids)}, "objects": objects, "themes": {"theme:brand": {"id": "theme:brand", "name": "MusterOffice synthetic", "colors": colors, "defaultText": {}}}, "masters": {"master:brand": {"id": "master:brand", "theme": "theme:brand", "objects": ["footer:master"], "background": solid("FFFFFF"), "defaultText": {}}}, "layouts": {"layout:brand": {"id": "layout:brand", "master": "master:brand", "name": "Brand blank", "objects": ["rule:layout"], "background": {"kind": "inherit"}, "defaultText": {}}}, "fonts": {}, "resources": {"resource:checker": {"id": "resource:checker", "kind": "picture", "sha256": hashlib.sha256(data).hexdigest(), "mediaType": "image/png"}}}
    request = {"document": document, "defaults": {"fontFamily": "Arial", "textSize": "304800", "textColor": rgba("162238"), "pageBackground": rgba("FFFFFF"), "themeColors": colors, "fontDelivery": "referenceOnly"}, "resourceBindings": [{"resourceId": "resource:checker", "byteOffset": "0", "byteLength": str(len(data))}]}
    (args.directory / "request.json").write_text(json.dumps(request, ensure_ascii=False, indent=2) + "\n")
    (args.directory / "resources.bin").write_bytes(data)
    print(json.dumps({"slides": len(document["slides"]), "objects": len(objects), "resourceBytes": len(data)}))


if __name__ == "__main__":
    main()
