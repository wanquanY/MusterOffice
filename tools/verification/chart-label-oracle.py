"""Independent source XML + rational arithmetic check of label binding plans.

Usage: python3 chart-label-oracle.py <cli> <cases.json> <new-report.json>
Does not verify display formatting, text placement or application rendering.
"""
from copy import deepcopy
from fractions import Fraction
from pathlib import Path
import hashlib
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

C = '{http://schemas.openxmlformats.org/drawingml/2006/chart}'
MC = '{http://schemas.openxmlformats.org/markup-compatibility/2006}'
FLAGS = dict(legendKey='showLegendKey', value='showVal', categoryName='showCatName',
             seriesName='showSerName', percent='showPercent', bubbleSize='showBubbleSize')


def check(source, request, response):
    plan = response['labels']
    assert hashlib.sha256(source).hexdigest() == request['expectedSourceSha256'] == plan['sourceSha256']
    assert plan['object'] == request['object']
    with zipfile.ZipFile(__import__('io').BytesIO(source)) as z:
        xml = z.read(plan['chartPart'].lstrip('/'))
    assert hashlib.sha256(xml).hexdigest() == plan['chartSha256']
    root = ET.fromstring(xml)
    assert not any(n.tag.startswith(MC) for n in root.iter()), 'MCE requires independent projection'
    physical = list(root.iter())
    ids = {n: i for i, n in enumerate(physical)}
    plot = physical[request['plotSourceOrdinal']]
    series = {int(s.find(C+'idx').get('val')): s for s in plot.findall(C+'ser')}
    normalized = set()
    components, fractions = 0, 0

    def selected(chain, name):
        return next((n.find(C+name) for n in chain if n.find(C+name) is not None), None)

    def boolean(n):
        assert n is not None
        return dict(value=n.get('val', 'true').strip() in {'1', 'true'},
                    sourceOrdinal=ids[n], schemaDefaulted='val' not in n.attrib)

    def cache(s, role):
        channel = s.find(C+role)
        assert channel is not None
        for kind in ['strLit', 'numLit', 'strRef', 'numRef']:
            v = channel.find(C+kind)
            if v is not None:
                c = v if kind.endswith('Lit') else v.find(C+('strCache' if kind == 'strRef' else 'numCache'))
                return channel, c, {int(p.get('idx')): p for p in c.findall(C+'pt')}, kind.startswith('num')
        raise AssertionError('unexpected data channel')

    def data_format(fmt, c, p):
        if fmt is not None and fmt.get('sourceLinked', 'true').strip() in {'0', 'false'}:
            return dict(sourceOrdinal=ids[fmt], code=fmt.get('formatCode'))
        if p.get('formatCode') is not None:
            return dict(sourceOrdinal=ids[p], code=p.get('formatCode'))
        f = c.find(C+'formatCode')
        return None if f is None else dict(sourceOrdinal=ids[c], code=f.text or '')

    assert len(plan['labels']) == len(request['targets'])
    for target, label in zip(request['targets'], plan['labels'], strict=True):
        assert label['target'] == target
        s = series[target['seriesIndex']]
        sg, pg = s.find(C+'dLbls'), plot.find(C+'dLbls')
        point = None if sg is None else next((p for p in sg.findall(C+'dLbl') if int(p.find(C+'idx').get('val')) == target['pointIndex']), None)
        chain = [n for n in [point, sg, pg] if n is not None]
        assert label['annotationChain'] == [ids[n] for n in chain]
        settings = label['settings']
        expected_flags = {role: boolean(n) for role, name in FLAGS.items() if (n := selected(chain, name)) is not None}
        assert settings['flags'] == expected_flags
        assert set(settings['unresolvedFlags']) == set(FLAGS)-set(expected_flags)
        deleted = selected(chain, 'delete')
        assert settings['deleted'] == (None if deleted is None else boolean(deleted))
        position = selected(chain, 'dLblPos')
        assert settings['position'] == (None if position is None else dict(
            value=position.get('val'), sourceOrdinal=ids[position], schemaDefaulted=False))
        leaders = selected(chain, 'showLeaderLines')
        assert settings['showLeaderLines'] == (None if leaders is None else boolean(leaders))
        layout = selected(chain, 'layout')
        assert settings['layoutSourceOrdinal'] == (None if layout is None else ids[layout])
        fmt = selected(chain, 'numFmt')
        assert settings['numberFormat'] == (None if fmt is None else dict(
            sourceOrdinal=ids[fmt], code=fmt.get('formatCode'), sourceLinked=dict(
                sourceOrdinal=ids[fmt], value=fmt.get('sourceLinked', 'true').strip() in {'true', '1'},
                schemaDefaulted='sourceLinked' not in fmt.attrib)))
        txpr = [ids[n.find(C+'txPr')] for n in chain if n.find(C+'txPr') is not None]
        txpr += [ids[n] for n in root.findall(C+'txPr')]
        assert settings['textPropertyRoots'] == txpr
        assert settings['shapePropertyRoots'] == [ids[n.find(C+'spPr')] for n in chain if n.find(C+'spPr') is not None]
        sep = selected(chain, 'separator')
        flags = {k: v['value'] for k, v in expected_flags.items()}
        if sep is not None:
            expected_sep = dict(kind='declared', value=sep.text or '', sourceOrdinal=ids[sep])
        elif len(flags) < 6:
            expected_sep = None
        elif plot.tag in {C+'pieChart', C+'pie3DChart', C+'ofPieChart'} and {k for k, v in flags.items() if v} == {'categoryName', 'percent'}:
            expected_sep = dict(kind='pieCategoryPercentLineBreak')
        else:
            expected_sep = dict(kind='commaDefault')
        assert settings['separator'] == expected_sep
        custom = selected(chain, 'tx')
        assert label['customTextSource'] == (None if custom is None else ids[custom])
        is_deleted = deleted is not None and boolean(deleted)['value']
        if is_deleted or flags.get('legendKey') is False:
            key = False
        elif flags.get('legendKey') and (custom is not None or any(v for k, v in flags.items() if k != 'legendKey')):
            key = True
        elif flags.get('legendKey') and len(flags) == 6:
            key = False
        else:
            key = None
        assert label['legendKeyVisible'] == key
        expected_components = []
        if not is_deleted and custom is None:
            for role, native_role in [('seriesName', 'tx'), ('categoryName', 'cat'), ('value', 'val'), ('bubbleSize', 'bubbleSize')]:
                if not flags.get(role):
                    continue
                if role == 'value' and s.find(C+'val') is None:
                    native_role = 'yVal'
                channel = s.find(C+native_role)
                literal = channel.find(C+'v') if channel is not None else None
                if role == 'seriesName' and literal is not None:
                    expected_components.append(dict(kind='text', role=role, sourceOrdinal=ids[channel], channelSourceOrdinal=ids[channel], value=literal.text or ''))
                    continue
                channel, c, points, numeric = cache(s, native_role)
                p = points[0 if role == 'seriesName' else target['pointIndex']]
                raw = p.find(C+'v').text or ''
                item = dict(kind='number' if numeric else 'text', role=role, sourceOrdinal=ids[p], channelSourceOrdinal=ids[channel], value=raw)
                if numeric:
                    item['format'] = data_format(fmt, c, p)
                expected_components.append(item)
            if flags.get('percent'):
                norm_index = next(i for i, n in enumerate(plan['normalizations']) if n['seriesIndex'] == target['seriesIndex'])
                norm = plan['normalizations'][norm_index]
                channel, c, points, _ = cache(s, 'val')
                assert norm['channelSourceOrdinal'] == ids[channel]
                ratio = norm['ratios']
                weights = {i: Fraction(p.find(C+'v').text) for i, p in points.items()}
                if request['negativeWeights'] == 'absoluteMagnitude':
                    weights = {i: abs(w) for i, w in weights.items()}
                assert all(w >= 0 for w in weights.values())
                total = sum(weights.values())
                assert total > 0 and not ratio['zeroTotal']
                numerators = {n['pointIndex']: int(n['numerator']) for n in ratio['numerators']}
                assert set(numerators) == set(weights)
                denominator = int(ratio['denominator'])
                assert sum(numerators.values()) == denominator
                for i, w in weights.items():
                    assert Fraction(numerators[i], denominator) == w/total
                if norm_index not in normalized:
                    fractions += len(weights)
                    normalized.add(norm_index)
                format_code = None if fmt is None or fmt.get('sourceLinked', 'true') not in {'0', 'false'} else dict(sourceOrdinal=ids[fmt], code=fmt.get('formatCode'))
                expected_components.append(dict(kind='percent', normalization=norm_index, pointIndex=target['pointIndex'], format=format_code))
        assert label['components'] == expected_components
        components += len(expected_components)
    assert normalized == set(range(len(plan['normalizations'])))
    return dict(labels=len(plan['labels']), components=components, exactFractions=fractions)


def main():
    cli, cases_file, output = sys.argv[1:]
    records, rejected = [], 0
    for c in json.loads(Path(cases_file).read_text()):
        request = json.loads(Path(c['request']).read_text())
        source = Path(c['source']).read_bytes()
        response = json.loads(subprocess.check_output([cli, 'pptx-chart-labels', c['request'], c['source']]))
        assert response['status'] == c['expectedStatus'], c['name']
        record = dict(name=c['name'], status=response['status'])
        if response['status'] == 'planned':
            record.update(check(source, request, response))
            controls = []
            if response['labels']['labels'][0]['settings']['flags']:
                bad = deepcopy(response)
                v = next(iter(bad['labels']['labels'][0]['settings']['flags'].values()))
                v['value'] = not v['value']
                controls.append(bad)
            if response['labels']['normalizations']:
                bad = deepcopy(response)
                ratio = bad['labels']['normalizations'][0]['ratios']
                ratio['denominator'] = str(int(ratio['denominator'])+1)
                controls.append(bad)
            for bad in controls:
                try:
                    check(source, request, bad)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError('oracle admitted altered label/ratio')
        records.append(record)
    report = dict(profile='chart-label-independent-xml-fraction-oracle/1', cases=records,
                  rejectedMutations=rejected, formattedLabelsProven=False, renderingProven=False, officeWpsProven=False)
    with Path(output).open('x') as f:
        json.dump(report, f, indent=2)
        f.write('\n')
    print(json.dumps(dict(cases=len(records), rejectedMutations=rejected)))


if __name__ == '__main__':
    main()
