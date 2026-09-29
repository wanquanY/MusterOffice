"""Independently compare source chart declarations with actual package XML.

Usage: python3 pptx-chart-layout-check.py CLI CASES_JSON NEW_REPORT_JSON
Cases use the same caller-owned input list as pptx-chart-parity.mjs. This checks
plain source trees, not MCE projection, computed geometry or target-app fidelity.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

C = "{http://schemas.openxmlformats.org/drawingml/2006/chart}"
PLOT = dict(barDir="barDirection", grouping="grouping", varyColors="varyColors",
            gapWidth="gapWidth", gapDepth="gapDepth", overlap="overlap",
            firstSliceAng="firstSliceAngle", holeSize="holeSize")
AXIS = dict(delete="delete", axPos="axisPosition", majorTickMark="majorTickMark",
            minorTickMark="minorTickMark", tickLblPos="tickLabelPosition", crossAx="crossAxis",
            crosses="crosses", crossesAt="crossesAt", crossBetween="crossBetween",
            majorUnit="majorUnit", minorUnit="minorUnit", auto="auto", lblAlgn="labelAlignment",
            lblOffset="labelOffset", tickLblSkip="tickLabelSkip", tickMarkSkip="tickMarkSkip",
            noMultiLvlLbl="noMultiLevelLabels", baseTimeUnit="baseTimeUnit",
            majorTimeUnit="majorTimeUnit", minorTimeUnit="minorTimeUnit")
SCALING = dict(logBase="logBase", orientation="orientation", min="minimum", max="maximum")
AXIS_KINDS = dict(catAx="category", valAx="value", dateAx="date", serAx="series")
PLOT_MARKUP = dict(dLbls="dataLabels", serLines="seriesLines")
AXIS_MARKUP = dict(spPr="shapeProperties", txPr="textProperties", title="title",
                   majorGridlines="majorGridlines", minorGridlines="minorGridlines", dispUnits="displayUnits")


def main():
    cli, cases_file, report_file = sys.argv[1:]
    records = []
    for case in json.loads(Path(cases_file).read_text()):
        result = json.loads(subprocess.check_output([cli, "pptx-charts", case["request"], case["source"]]))
        assert result["status"] == case["expectedStatus"]
        if result["status"] != "inspected":
            records.append(dict(name=case["name"], status="error", code=result["error"]["code"]))
            continue
        checked = 0
        axis_count = 0
        unresolved = 0
        with zipfile.ZipFile(case["source"]) as package:
            for chart in result["charts"]["charts"]:
                data = package.read(chart["part"].lstrip("/"))
                assert hashlib.sha256(data).hexdigest() == chart["sha256"]
                root = ET.fromstring(data)
                # MCE requires an independent projection; do not silently treat raw
                # branch children as the selected semantic tree.
                assert not any(n.tag.startswith("{http://schemas.openxmlformats.org/markup-compatibility/2006}") for n in root.iter())
                nodes = list(root.iter())
                ordinal = {node: i for i, node in enumerate(nodes)}

                def properties(parent, mapping):
                    return [dict(sourceOrdinal=ordinal[n], kind=mapping[n.tag[len(C):]], value=n.get("val"))
                            for n in parent if n.tag.startswith(C) and n.tag[len(C):] in mapping]

                def markup(parent, mapping):
                    return [dict(sourceOrdinal=ordinal[n], kind=mapping[n.tag[len(C):]])
                            for n in parent if n.tag.startswith(C) and n.tag[len(C):] in mapping]

                area = root.find(C + "chart/" + C + "plotArea")
                expected_axes = [n for n in area if n.tag.startswith(C) and n.tag[len(C):] in AXIS_KINDS]
                assert len(expected_axes) == len(chart["axes"])
                ids = set()
                for xml, axis in zip(expected_axes, chart["axes"], strict=True):
                    assert axis["sourceOrdinal"] == ordinal[xml]
                    assert axis["kind"] == AXIS_KINDS[xml.tag[len(C):]]
                    assert axis["id"] == int(xml.find(C + "axId").get("val"))
                    ids.add(axis["id"])
                    assert axis["layout"] == dict(properties=properties(xml, AXIS), markup=markup(xml, AXIS_MARKUP))
                    scaling = xml.find(C + "scaling")
                    expected = None if scaling is None else dict(sourceOrdinal=ordinal[scaling], properties=properties(scaling, SCALING))
                    assert axis["scaling"] == expected
                    number_format = xml.find(C + "numFmt")
                    expected = None if number_format is None else dict(sourceOrdinal=ordinal[number_format], formatCode=number_format.get("formatCode"), sourceLinked=number_format.get("sourceLinked"))
                    assert axis["numberFormat"] == expected
                    checked += len(axis["layout"]["properties"]) + len(axis["layout"]["markup"])
                    checked += len(axis["scaling"]["properties"]) if axis["scaling"] else 0
                    axis_count += 1
                for plot in chart["plots"]:
                    xml = nodes[plot["sourceOrdinal"]]
                    assert xml in list(area) and xml.tag == C + plot["nativeKind"]
                    assert plot["layout"] == dict(properties=properties(xml, PLOT), markup=markup(xml, PLOT_MARKUP))
                    refs = [int(n.get("val")) for n in xml.findall(C + "axId")]
                    assert plot["axisIds"] == refs
                    unresolved += sum(i not in ids for i in refs)
                    checked += len(plot["layout"]["properties"]) + len(plot["layout"]["markup"])
        records.append(dict(name=case["name"], status="inspected", checkedDeclarations=checked,
                            axes=axis_count, unresolvedPlotAxisReferences=unresolved))
    report = dict(profile="pptx-chart-layout-independent-xml/1", cases=records,
                  renderingProven=False, workbookConsistencyProven=False, officeWpsProven=False)
    with Path(report_file).open("x") as file:
        json.dump(report, file, indent=2)
        file.write("\n")
    print(json.dumps(dict(cases=len(records), declarations=sum(r.get("checkedDeclarations", 0) for r in records))))


if __name__ == "__main__":
    main()
