"""Paced complete PPTX files against independent Fraction clocks and static angles.

This verifies native export/read/render and supplies actual WASM/product inputs.
It does not imply external Office/WPS playback or complete PPT acceptance.
"""
import argparse
import copy
from fractions import Fraction
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as X
import zipfile
from lxml import etree

spec = importlib.util.spec_from_file_location('rotation', Path(__file__).with_name('rotation-composition-fixtures.py'))
rotation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rotation)
f = rotation.f
NAMES = ['rate-parallel', 'rate-sequence', 'rate-nested', 'rate-easing']


def at(value):
    value = Fraction(value)
    return dict(kind='at', offset=f.time(value.numerator, value.denominator))


def group(name, speed, children, start=0):
    return dict(id=name, kind='parallel', start=at(start), duration=dict(kind='automatic'),
                fill='hold', children=children, timeTransform=dict(speedMilliPercent=speed,
                autoReverse=False, accelerationMilliPercent=0, decelerationMilliPercent=0))


def timeline(name):
    nodes = []
    for i in range(6):
        delay = Fraction(i, 2) if name in ['rate-parallel', 'rate-nested'] else Fraction(i % 2, 2)
        duration = 2 + i if name in ['rate-parallel', 'rate-nested'] else 2
        if name == 'rate-easing':
            delay, duration = 0, 6
        nodes.append(dict(id='spin-' + str(i), start=at(delay), duration=f.time(duration),
                          repeatMilli=1000, fill='hold', effect=dict(kind='rotation',
                          target='shape:' + str(i), composition='absolute',
                          **{'from': 20 * 60000, 'to': (40 + 20 * i) * 60000})))
    outer = group('outer', 200000, [n['id'] for n in nodes], 1)
    containers = [outer]
    if name == 'rate-sequence':
        outer['kind'] = 'sequence'
    elif name == 'rate-nested':
        inner = group('inner', 150000, outer['children'], 1)
        outer['children'] = ['inner']
        containers.append(inner)
    elif name == 'rate-easing':
        inner = group('inner', 50000, outer['children'])
        inner['timeTransform'].update(accelerationMilliPercent=50000, decelerationMilliPercent=50000)
        middle = group('middle', 150000, ['inner'])
        outer['children'] = ['middle']
        outer['timeTransform']['accelerationMilliPercent'] = 100000
        containers.extend([middle, inner])
    return dict(format='musteroffice.timeline/0.2-draft', nodes=nodes,
                tree=dict(roots=['outer'], containers=containers))


def expected(name, index, time):
    # Independent closed forms for host start, duration and progress. These do
    # not read the computed timing tree or reuse the kernel's clock projection.
    if name == 'rate-parallel':
        start, span = 1 + Fraction(index, 4), Fraction(2 + index, 2)
    elif name == 'rate-nested':
        start, span = Fraction(3, 2) + Fraction(index, 6), Fraction(2 + index, 3)
    elif name == 'rate-sequence':
        start = 1 + index + sum(Fraction(j % 2, 4) for j in range(index + 1))
        span = 1
    else:
        start, span = 1, 4
    if time < start:
        return None, Fraction(20)
    p = min(Fraction(1), (time - start) / span)
    if name == 'rate-easing':
        p *= p
        p = 2*p*p if p <= Fraction(1, 2) else 1-2*(1-p)**2
    return p, 20 + (20 + 20 * index) * p


def main(names=NAMES, make_timeline=timeline, reference=expected, extra_inputs=(),
         sample_times=None, check_state=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['output', 'export-template', 'page-template', 'worker', 'cli', 'schema']:
        parser.add_argument('--' + key, type=Path, required=True)
    a = parser.parse_args()
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {}

    def tracked(path):
        record = f.entry(path.resolve())
        inputs[record['path']] = record
        return f.load(record)

    for path in [a.worker, a.cli, Path(__file__), Path(rotation.__file__), Path(f.__file__), *extra_inputs]:
        tracked(path)
    for path in a.schema.parent.glob('*.xsd'):
        tracked(path)
    schema = etree.XMLSchema(etree.parse(str(a.schema)))
    export_template = json.loads(tracked(a.export_template))
    page_template = json.loads(tracked(a.page_template))
    fonts = f.put(out / 'empty.bin', b'')
    times = [Fraction(x) for x in (sample_times if sample_times is not None else
             ['0','0.25','0.5','1','1.125','1.25','1.5','1.75','2','2.5','3','4','5','8','20','1/7','7/3','0.5'])]
    cases, checks, controls, xsd_parts = [], [], [], []
    for name in names:
        request = copy.deepcopy(export_template)
        request['document']['timelines']['slide:1'] = make_timeline(name)
        path = f.put(out / (name + '.export.json'), request)
        source_path = out / (name + '.pptx')
        result = subprocess.run([str(a.cli.resolve()), 'pptx-export', path['path'], fonts['path'], str(source_path)], capture_output=True, timeout=60)
        assert result.returncode == 0 and not result.stderr, result.stderr
        f.put(out / (name + '.export-response.json'), result.stdout)
        source = f.entry(source_path)
        raw = f.load(source)
        with zipfile.ZipFile(io.BytesIO(raw)) as package:
            parts = {part: package.read(part) for part in package.namelist()}
        for part, data in parts.items():
            if not part.endswith('.xml'):
                continue
            root = etree.fromstring(data)
            if root.tag.startswith('{' + f.P + '}'):
                schema.assertValid(root)
                xsd_parts.append(dict(source=source, part=part))
        slide = 'ppt/slides/slide1.xml'
        tree = X.fromstring(parts[slide])
        tree.remove(tree.find('p:timing', f.NS))
        shapes = tree.findall('p:cSld/p:spTree/p:sp', f.NS)
        assert len(shapes) == 6
        native_ids = [shape.find('p:nvSpPr/p:cNvPr', f.NS).get('id') for shape in shapes]
        for index, time in enumerate(times):
            q = copy.deepcopy(page_template)
            q['page']['page']['expectedSourceSha256'] = source['sha256']
            q['sample']['binding'] = dict(session=name, revision=source['sha256'], generation='1')
            q['sample']['history'] = None
            q['sample']['at'] = f.time(time.numerator, time.denominator)
            response, metadata, pixels = f.native(a.worker, q, raw, b'')
            angles = [reference(name, i, time) for i in range(6)]
            rotations = {'sp.' + native_ids[i]: dict(**f.exact((angle-20)*60000), basis='layout')
                         for i, (progress, angle) in enumerate(angles) if progress is not None}
            state = response['info']['playback']['evaluated']['state']
            if check_state is not None:
                check_state(name, time, state)
            assert state['rotations'] == rotations, (name, str(time), state['rotations'], rotations)
            stem = name + '-' + str(index)
            cases.append(dict(name=stem, request=f.put(out / (stem+'.request.json'), q),
                              response=f.put(out / (stem+'.response.json'), metadata),
                              pixels=f.put(out / (stem+'.rgba'), pixels), source=source, fonts=fonts))
            checks.append(dict(name=stem, time=str(time), expectedRotations=rotations))
            if all((angle*60000).denominator == 1 for _, angle in angles):
                control = copy.deepcopy(tree)
                for shape, (_, angle) in zip(control.findall('p:cSld/p:spTree/p:sp', f.NS), angles):
                    shape.find('p:spPr/a:xfrm', f.NS).set('rot', str(int(angle*60000)))
                static_parts = dict(parts)
                static_parts[slide] = X.tostring(control)
                static = rotation.archive(static_parts)
                qc = copy.deepcopy(q['page'])
                qc['page']['expectedSourceSha256'] = f.sha(static)
                _, _, expected_pixels = f.native(a.worker, qc, static, b'', '--pptx-resource-page')
                assert pixels == expected_pixels, stem + ' independent static geometry'
                controls.append(dict(name=stem, source=f.put(out / (stem+'.control.pptx'), static),
                                     request=f.put(out / (stem+'.control.json'), qc), pixelSha256=f.sha(expected_pixels)))
    f.put(out/'source.json', dict(cases=cases))
    f.put(out/'author.json', dict(cases=[]))
    for record in inputs.values():
        f.load(record)
    f.put(out/'report.json', dict(status='passed', inputs=inputs, frames=len(cases), checks=checks,
                                staticControls=controls, xsdParts=xsd_parts))
    print(json.dumps(dict(status='passed', frames=len(cases), staticControls=len(controls), xsdParts=len(xsd_parts))))


if __name__ == '__main__':
    main()
