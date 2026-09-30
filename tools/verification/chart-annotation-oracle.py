"""Independently bind chart annotations/text to original ZIP/XML.

Usage: python3 chart-annotation-oracle.py <cli> <cases.json> <new-report.json>
This oracle covers source declarations, not inheritance, rendering or Office.
MCE sources must use a separate projection oracle; they fail here explicitly.
"""
from copy import deepcopy
from pathlib import Path
import hashlib
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

C = '{http://schemas.openxmlformats.org/drawingml/2006/chart}'
A = '{http://schemas.openxmlformats.org/drawingml/2006/main}'
MC = '{http://schemas.openxmlformats.org/markup-compatibility/2006}'
KINDS = dict(dLbls='dataLabels', dLbl='dataLabel', legend='legend',
             legendEntry='legendEntry', title='title', layout='layout', manualLayout='manualLayout')
PROPERTIES = dict(delete='delete', dLblPos='labelPosition', showLegendKey='showLegendKey',
                  showVal='showValue', showCatName='showCategoryName', showSerName='showSeriesName',
                  showPercent='showPercent', showBubbleSize='showBubbleSize', separator='separator',
                  showLeaderLines='showLeaderLines', legendPos='legendPosition', overlay='overlay',
                  layoutTarget='layoutTarget', xMode='xMode', yMode='yMode', wMode='widthMode',
                  hMode='heightMode', x='x', y='y', w='width', h='height')
PLOTS = {'areaChart', 'area3DChart', 'lineChart', 'line3DChart', 'stockChart', 'radarChart',
         'scatterChart', 'pieChart', 'pie3DChart', 'doughnutChart', 'barChart', 'bar3DChart',
         'ofPieChart', 'surfaceChart', 'surface3DChart', 'bubbleChart'}
AXES = {'catAx', 'valAx', 'dateAx', 'serAx'}


def verify_chart(data, result):
    assert hashlib.sha256(data).hexdigest() == result['sha256']
    root = ET.fromstring(data)
    assert not any(n.tag.startswith(MC) for n in root.iter()), 'MCE needs independent projection'
    physical = list(root.iter())
    ids = {n: i for i, n in enumerate(physical)}
    parents = {child: parent for parent in physical for child in parent}
    annotations = []
    for n in physical:
        if not n.tag.startswith(C) or n is root:
            continue
        name, parent = n.tag.removeprefix(C), parents[n]
        pname = parent.tag.removeprefix(C)
        if ((pname == 'chart' and name in {'legend', 'title'})
                or (pname == 'plotArea' and name == 'layout')
                or (pname in AXES and name == 'title')
                or (pname in PLOTS | {'ser'} and name == 'dLbls')
                or (parent in annotations and (
                    (pname == 'dLbls' and name == 'dLbl')
                    or (pname == 'legend' and name == 'legendEntry')
                    or (pname in {'title', 'legend', 'dLbl'} and name == 'layout')
                    or (pname == 'layout' and name == 'manualLayout')))):
            annotations.append(n)
    observed = result.get('annotations', {'nodes': [], 'textBodies': []})
    assert len(observed['nodes']) == len(annotations)
    declarations = 0
    for n, record in zip(annotations, observed['nodes'], strict=True):
        assert record['sourceOrdinal'] == ids[n]
        assert record['parentOrdinal'] == ids[parents[n]]
        assert record['kind'] == KINDS[n.tag.removeprefix(C)]
        idx = n.find(C+'idx')
        assert record['index'] == (None if idx is None else int(idx.get('val')))
        expected = [dict(sourceOrdinal=ids[c], kind=PROPERTIES[c.tag.removeprefix(C)],
                         value=(''.join(c.itertext()) if c.tag == C+'separator' else c.get('val')))
                    for c in n if c.tag.startswith(C) and c.tag.removeprefix(C) in PROPERTIES]
        assert record['declarations']['properties'] == expected
        declarations += len(expected)
        fmt = n.find(C+'numFmt')
        assert record['numberFormat'] == (None if fmt is None else dict(
            sourceOrdinal=ids[fmt], formatCode=fmt.get('formatCode'), sourceLinked=fmt.get('sourceLinked')))
        tx = n.find(C+'tx')
        if tx is None:
            assert record['textSource'] is None
        else:
            assert record['textSource']['sourceOrdinal'] == ids[tx]
            rich = tx.find(C+'rich')
            if rich is not None:
                assert record['textSource']['content'] == dict(kind='rich', sourceOrdinal=ids[rich])
    bodies = [n for n in physical if (
        (n.tag == C+'txPr' and (parents[n] is root or parents[n] in annotations
                               or parents[n].tag.removeprefix(C) in AXES))
        or (n.tag == C+'rich' and parents[n].tag == C+'tx'
            and parents.get(parents[n]) in annotations))]
    assert len(observed['textBodies']) == len(bodies)
    runs = 0
    font_declarations = 0
    for xml, body in zip(bodies, observed['textBodies'], strict=True):
        assert body['sourceOrdinal'] == ids[xml]
        assert body['parentOrdinal'] == ids[parents[xml]]
        styles = body['styles']
        assert len(styles['roots']) == 1
        assert styles['roots'][0]['sourceOrdinal'] == ids[xml]
        assert styles['roots'][0]['owner'] is None
        assert styles['nodes'][str(ids[xml])]['element'] == xml.tag.removeprefix(C)
        expected_fonts = {ids[n] for n in xml.iter() if n.tag in {A+'latin', A+'ea', A+'cs', A+'sym'}}
        actual_fonts = set()
        for ordinal, style in styles['nodes'].items():
            n = physical[int(ordinal)]
            assert style['element'] == n.tag.split('}')[-1]
            assert style['parent'] == (None if n is xml else ids[parents[n]])
            if 'sz' in n.attrib:
                assert style['value']['attributes']['size'] == int(n.get('sz'))
                font_declarations += 1
            if style['element'] in {'latin', 'ea', 'cs', 'sym'}:
                actual_fonts.add(int(ordinal))
                assert style['value']['font']['typeface'] == n.get('typeface')
        assert actual_fonts == expected_fonts
        paragraphs = xml.findall(A+'p')
        assert len(body['paragraphs']) == len(paragraphs)
        for p, paragraph in zip(paragraphs, body['paragraphs'], strict=True):
            assert paragraph['sourceOrdinal'] == ids[p]
            native = [n for n in p if n.tag in {A+'r', A+'br', A+'fld'}]
            assert len(native) == len(paragraph['runs'])
            for n, run in zip(native, paragraph['runs'], strict=True):
                t = n.find(A+'t')
                assert run == dict(sourceOrdinal=ids[n], textSourceOrdinal=None if t is None else ids[t],
                                   kind={A+'r': 'text', A+'br': 'break', A+'fld': 'field'}[n.tag],
                                   text='' if t is None else ''.join(t.itertext()))
                runs += 1
    return dict(annotations=len(annotations), bodies=len(bodies), declarations=declarations,
                runs=runs, explicitFontSizes=font_declarations)


def main():
    cli, cases_file, output = sys.argv[1:]
    cases = json.loads(Path(cases_file).read_text())
    records, negative_controls = [], 0
    for case in cases:
        response = json.loads(subprocess.check_output([cli, 'pptx-charts', case['request'], case['source']]))
        assert response['status'] == case['expectedStatus']
        if response['status'] != 'inspected':
            records.append(dict(name=case['name'], status=response['status']))
            continue
        with zipfile.ZipFile(case['source']) as z:
            checked = []
            for chart in response['charts']['charts']:
                data = z.read(chart['part'].lstrip('/'))
                checked.append(verify_chart(data, chart))
                # Changing a real origin or declaration must fail the XML oracle.
                if chart.get('annotations', {}).get('nodes'):
                    bad = deepcopy(chart)
                    bad['annotations']['nodes'][0]['parentOrdinal'] += 1
                    try:
                        verify_chart(data, bad)
                    except AssertionError:
                        negative_controls += 1
                    else:
                        raise AssertionError('oracle admitted wrong parent')
                if chart.get('annotations', {}).get('textBodies'):
                    bad = deepcopy(chart)
                    b = bad['annotations']['textBodies'][0]
                    changed = False
                    for n in b['styles']['nodes'].values():
                        attrs = n['value'].get('attributes', {})
                        if attrs.get('size') is not None:
                            attrs['size'] += 100
                            changed = True
                            break
                    if changed:
                        try:
                            verify_chart(data, bad)
                        except AssertionError:
                            negative_controls += 1
                        else:
                            raise AssertionError('oracle admitted wrong font size')
        records.append(dict(name=case['name'], status=response['status'], charts=checked,
                            sourceSha256=hashlib.sha256(Path(case['source']).read_bytes()).hexdigest()))
    report = dict(profile='chart-annotation-source-oracle/1', cases=records,
                  rejectedMutations=negative_controls, renderingProven=False, officeWpsProven=False)
    with Path(output).open('x') as f:
        json.dump(report, f, indent=2)
        f.write('\n')
    print(json.dumps(dict(cases=len(records), rejectedMutations=negative_controls)))


if __name__ == '__main__':
    main()
