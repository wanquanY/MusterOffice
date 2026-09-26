"""Independent tree-based MCE oracle for the owned PPTX source corpus.

Uses lxml rather than the kernel's streaming XML/events. This is a test oracle
for successful inputs in the current PML profile, not a general conformance tool.
It checks branch selection, source ordinals, scope inheritance, logical objects
and text. The separate byte-preservation oracle checks actual mutation outputs.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree

P = "http://schemas.openxmlformats.org/presentationml/2006/main"
A = "http://schemas.openxmlformats.org/drawingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
XML = "http://www.w3.org/XML/1998/namespace"
UNDERSTOOD = {P, A, R, XML, ""}
NS = {"p": P, "a": A}
EXTENSIONS = {f"{{{ns}}}extLst" for ns in [P, A]}


def expanded(name):
    qname = etree.QName(name)
    return qname.namespace or "", qname.localname


def project(raw, with_ordinals=False):
    parser = etree.XMLParser(resolve_entities=False, no_network=True, remove_comments=True, remove_pis=True)
    root = etree.fromstring(raw, parser)
    ordinals = {node: i for i, node in enumerate(root.iter())}
    projected_ordinals = {}
    summary = {"selections": [], "ignoredElements": 0, "ignoredAttributes": 0, "unwrappedElements": 0}

    def prefixes(node, value):
        return [XML if p == "xml" else node.nsmap[p] for p in value.split()]

    def rules(node, ignorable, process):
        ignorable = ignorable | set(prefixes(node, node.get(f"{{{MC}}}Ignorable", "")))
        process = set(process)
        for token in node.get(f"{{{MC}}}ProcessContent", "").split():
            prefix, local = token.split(":")
            namespace = node.nsmap[prefix]
            assert namespace in ignorable
            process.add((namespace, local))
        return ignorable, process

    def require(node):
        assert set(prefixes(node, node.get(f"{{{MC}}}MustUnderstand", ""))) <= UNDERSTOOD

    def contents(node, ignorable, process):
        result = [node.text] if node.text else []
        for child in node:
            result.extend(visit(child, ignorable, process))
            if child.tail:
                result.append(child.tail)
        return result

    def visit(node, ignorable, process):
        if node.tag in EXTENSIONS:
            result = copy.deepcopy(node); result.tail = None
            projected_ordinals.update((clone, ordinals[original]) for original, clone in zip(node.iter(), result.iter()))
            return [result]
        ignorable, process = rules(node, ignorable, process)
        namespace, local = expanded(node.tag)
        if namespace in ignorable and namespace not in UNDERSTOOD:
            if (namespace, local) in process or (namespace, "*") in process:
                require(node)
                summary["unwrappedElements"] += 1
                return contents(node, ignorable, process)
            summary["ignoredElements"] += 1
            return []
        require(node)
        if node.tag == f"{{{MC}}}AlternateContent":
            selection = {"sourceOrdinal": ordinals[node], "branches": []}
            summary["selections"].append(selection)
            selected = None
            for branch in node:
                if branch.tag not in {f"{{{MC}}}Choice", f"{{{MC}}}Fallback"}:
                    assert not visit(branch, ignorable, process)
                    continue
                fallback = branch.tag == f"{{{MC}}}Fallback"
                requires = [] if fallback else prefixes(branch, branch.get("Requires"))
                active = selected is None and (fallback or set(requires) <= UNDERSTOOD)
                selection["branches"].append({"sourceOrdinal": ordinals[branch], "requires": requires,
                                               "fallback": fallback, "selected": active})
                if active:
                    selected = branch
            if selected is None:
                return []
            require(selected)
            return contents(selected, *rules(selected, ignorable, process))
        result = etree.Element(node.tag, nsmap=node.nsmap)
        projected_ordinals[result] = ordinals[node]
        for name, value in node.attrib.items():
            ns, _ = expanded(name)
            if ns == MC:
                continue
            if ns in ignorable and ns not in UNDERSTOOD:
                summary["ignoredAttributes"] += 1
            else:
                result.set(name, value)
        for item in contents(node, ignorable, process):
            if isinstance(item, str):
                if len(result):
                    result[-1].tail = (result[-1].tail or "") + item
                else:
                    result.text = (result.text or "") + item
            else:
                result.append(item)
        return [result]

    projected = [item for item in visit(root, set(), set()) if not isinstance(item, str)]
    assert len(projected) == 1
    if with_ordinals:
        return projected[0], summary, projected_ordinals
    return projected[0], summary


def objects(root):
    output = []
    kinds = {"sp": ("shape", "nvSpPr"), "pic": ("picture", "nvPicPr"),
             "grpSp": ("group", "nvGrpSpPr"), "cxnSp": ("connector", "nvCxnSpPr"),
             "graphicFrame": ("graphicFrame", "nvGraphicFramePr")}

    def collect(tree, parent):
        for node in tree:
            ns, local = expanded(node.tag)
            if ns != P or local not in kinds:
                continue
            kind, nv = kinds[local]
            identity = node.find(f"p:{nv}/p:cNvPr", NS)
            paragraphs = []
            for paragraph in node.findall("p:txBody/a:p", NS):
                runs = []
                for run in paragraph:
                    if run.tag in {f"{{{A}}}r", f"{{{A}}}fld", f"{{{A}}}br"}:
                        leaf = run.find("a:t", NS)
                        run_kind = {"r": "text", "fld": "field", "br": "break"}[expanded(run.tag)[1]]
                        runs.append({"kind": run_kind, "text": "" if leaf is None else "".join(leaf.itertext())})
                paragraphs.append(runs)
            obj = {"nativeId": int(identity.get("id")), "name": identity.get("name"), "kind": kind,
                   "parentGroup": parent, "paragraphs": paragraphs}
            output.append(obj)
            if kind == "group":
                collect(node, obj["nativeId"])
    collect(root.find("p:cSld/p:spTree", NS), None)
    return output


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity_report", type=Path)
    args = parser.parse_args()
    report = json.loads(args.parity_report.read_text())
    cases = []
    for case in report["cases"]:
        response = case.get("response", {})
        if response.get("status") != "inspected":
            continue
        index = response["index"]
        raw = Path(case["source"]).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == index["sourceSha256"]
        with zipfile.ZipFile(case["source"]) as package:
            main, summary = project(package.read(index["mainPart"].lstrip("/")))
            assert summary == index["mainCompatibility"], case["name"]
            assert [int(n.get("id")) for n in main.findall("p:sldIdLst/p:sldId", NS)] == [s["nativeId"] for s in index["slides"]]
            total = 0
            for part, surface in index["surfaces"].items():
                root, summary = project(package.read(part.lstrip("/")))
                assert summary == surface["compatibility"], (case["name"], part, summary)
                actual = [{k: o[k] for k in ["nativeId", "name", "kind", "parentGroup"]}
                          | {"paragraphs": [[{k: r[k] for k in ["kind", "text"]} for r in p] for p in o["paragraphs"]]}
                          for o in surface["objects"]]
                expected = objects(root)
                assert expected == actual, (case["name"], part, expected, actual)
                total += len(actual)
        cases.append({"name": case["name"], "sourceSha256": index["sourceSha256"], "surfaces": len(index["surfaces"]), "objects": total})
    print(json.dumps({"format": "musteroffice.mce-reference/1", "passed": len(cases), "cases": cases,
                      "scope": "Independent tree projection of successful owned PML corpus; not full MCE conformance or rendering."}, indent=2))


if __name__ == "__main__":
    main()
