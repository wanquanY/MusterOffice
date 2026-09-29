"""Owned connected paths against independent converged arc-length references.

Writes complete native exports and native/author frame inputs for WASM replay.
This is computation evidence, not an Office/WPS interoperability verdict.
"""
import argparse
import copy
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import zipfile
import xml.etree.ElementTree as X
import numpy as np
from lxml import etree

spec = importlib.util.spec_from_file_location('motion', Path(__file__).with_name('motion-playback-fixtures.py'))
motion = importlib.util.module_from_spec(spec)
spec.loader.exec_module(motion)
f = motion.f


def point(x, y):
    return dict(x=str(x), y=str(y))


def line(x, y):
    return dict(kind='line', to=point(x, y))


def cubic(a, b, c):
    return dict(kind='cubic', control1=point(*a), control2=point(*b), to=point(*c))


PATHS = [
    [cubic((0, 0), (0, 0), (0.3, 0))],
    [line(0.03, 0), line(0.3, 0)],
    [line(0.15, 0), line(0.15, 0.15)],
    [cubic((0.1, 0.1), (0.2, -0.1), (0.3, 0))],
    [cubic((0.15, 0.1), (-0.15, 0.1), (0, 0))],
    [line(0.15, 0), line(0.15, -0.08), dict(kind='close')],
]


def reference(segments, resolution):
    """Uniform parameter integration; no production subdivision or table data."""
    points = [np.zeros((1, 2))]
    current = np.zeros(2)
    for s in segments:
        to = np.zeros(2) if s['kind'] == 'close' else np.array([float(s['to'][k]) for k in ['x', 'y']])
        if s['kind'] == 'cubic':
            a, b = [np.array([float(s[c][k]) for k in ['x', 'y']]) for c in ['control1', 'control2']]
            t = np.linspace(0, 1, resolution + 1)[1:, None]
            u = 1 - t
            points.append(u**3 * current + 3*u*u*t*a + 3*u*t*t*b + t**3*to)
        else:
            points.append(to[None, :])
        current = to
    points = np.vstack(points)
    lengths = np.r_[0, np.cumsum(np.linalg.norm(np.diff(points, axis=0), axis=1))]

    def sample(progress):
        at = progress * lengths[-1]
        i = max(1, min(len(points)-1, np.searchsorted(lengths, at)))
        gap = lengths[i] - lengths[i-1]
        u = (at-lengths[i-1])/gap if gap else 0
        return points[i-1] + u * (points[i]-points[i-1])
    return sample


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['output', 'export-template', 'page-template', 'author-template', 'cli', 'worker', 'schema']:
        parser.add_argument('--'+key, type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {}

    def tracked(path):
        record = f.entry(path.resolve())
        inputs[record['path']] = record
        return f.load(record)

    for path in [args.cli, args.worker, Path(__file__), Path(motion.__file__), Path(f.__file__), *args.schema.parent.glob('*.xsd')]:
        tracked(path)
    template = json.loads(tracked(args.export_template))
    source_template = json.loads(tracked(args.page_template))
    author_template = json.loads(tracked(args.author_template))
    schema = etree.XMLSchema(etree.parse(str(args.schema)))
    fonts = f.put(out/'empty.bin', b'')
    references = [[reference(p, n) for n in [65536, 131072]] for p in PATHS]
    cases = dict(author=[], source=[])
    checks, controls, xsd_parts = [], [], []
    times = [0, 125, 500, 1000, 1500, 2000, 2500, 3000, 3500, 3999, 4000, 5000, 6000, 7000, 8000, 500]
    for name, reverse in [('path-forward', False), ('path-reverse', True)]:
        export = copy.deepcopy(template)
        doc = export['document']
        nodes = []
        for i, segments in enumerate(PATHS):
            node = dict(id=f'curve:{i}', start=dict(kind='at', offset=f.time(0)), duration=f.time(4),
                        repeatMilli=1000, fill='hold', effect=dict(kind='motionPath', target=f'shape:{i}',
                        path=dict(**{'from': point(0, 0)}, segments=segments)))
            if reverse:
                node['timeTransform'] = dict(speedMilliPercent=100000, autoReverse=True,
                                             accelerationMilliPercent=0, decelerationMilliPercent=0)
            nodes.append(node)
        doc['timelines'] = {'slide:1': dict(format='musteroffice.timeline/0.1-draft', nodes=nodes)}
        snapshot = motion.initialize(args.cli, doc)
        request = f.put(out/(name+'.export.json'), export)
        output = out/(name+'.pptx')
        response = subprocess.run([str(args.cli), 'pptx-export', request['path'], fonts['path'], str(output)], capture_output=True, check=True)
        assert not response.stderr
        f.put(out/(name+'.export-response.json'), response.stdout)
        source = f.entry(output)
        raw = f.load(source)
        with zipfile.ZipFile(io.BytesIO(raw)) as package:
            parts = {part: package.read(part) for part in package.namelist()}
        for part, data in parts.items():
            if part.endswith('.xml'):
                xml = etree.fromstring(data)
                if xml.tag.startswith('{'+f.P+'}'):
                    schema.assertValid(xml)
                    xsd_parts.append(dict(source=source, part=part))
        slide = 'ppt/slides/slide1.xml'
        tree = X.fromstring(parts[slide])
        paths = tree.findall('.//p:animMotion', f.NS)
        assert len(paths) == 6 and ' C ' in paths[3].get('path') and ' Z ' in paths[5].get('path')
        tree.remove(tree.find('p:timing', f.NS))
        shapes = tree.findall('p:cSld/p:spTree/p:sp', f.NS)
        ids = ['sp.'+shape.find('p:nvSpPr/p:cNvPr', f.NS).get('id') for shape in shapes]
        for index, ms in enumerate(times):
            progress = min(ms / 4000, 2 if reverse else 1)
            if progress > 1:
                progress = 2-progress
            expected = [r[1](progress) for r in references]
            convergence = max(float(np.linalg.norm(r[0](progress)-r[1](progress))) for r in references)
            assert convergence < 1e-9, convergence
            q = copy.deepcopy(source_template)
            q['page']['page']['expectedSourceSha256'] = source['sha256']
            q['sample'] = dict(binding=dict(session=name, revision=source['sha256'], generation='1'), at=f.time(ms, 1000), history=None)
            aq = dict(playback=dict(snapshot=snapshot, slide='slide:1', binding=dict(session=name, revision=snapshot['revision'], generation='1'), at=f.time(ms, 1000), history=None), viewport=q['page']['page']['viewport'], defaults=author_template['defaults'])
            rendered = {}
            for kind, request, source_bytes in [('source', q, raw), ('author', aq, None)]:
                response, metadata, pixels = f.native(args.worker, request, source_bytes, b'' if source_bytes else None)
                state = response['info']['frame']['state'] if kind == 'author' else response['info']['playback']['evaluated']['state']
                assert state['profile'] == 'musteroffice.paced-motion-frame-q64/0.1-draft'
                errors = []
                for i, wanted in enumerate(expected):
                    offset = state['motion'][f'shape:{i}' if kind == 'author' else ids[i]]
                    actual = [int(offset[k]['numerator'])/int(offset[k]['denominator']) for k in ['x', 'y']]
                    errors.append(float(np.linalg.norm(np.array(actual)-wanted)))
                assert max(errors) < 3e-6, errors
                stem = f'{name}-{kind}-{index}'
                record = dict(name=stem, request=f.put(out/(stem+'.request.json'), request), response=f.put(out/(stem+'.response.json'), metadata), pixels=f.put(out/(stem+'.rgba'), pixels))
                if kind == 'source':
                    record.update(source=source, fonts=fonts)
                cases[kind].append(record)
                checks.append(dict(name=stem, timeMs=ms, maxNormalizedPositionError=max(errors), referenceConvergence=convergence))
                rendered[kind] = pixels
            assert rendered['source'] == rendered['author'], 'native/author raster'
            # Static geometry oracle rounds only at native integer EMU storage.
            control = copy.deepcopy(tree)
            for shape, offset in zip(control.findall('p:cSld/p:spTree/p:sp', f.NS), expected):
                origin = shape.find('p:spPr/a:xfrm/a:off', f.NS)
                for axis, dimension, value in zip(['x', 'y'], ['width', 'height'], offset):
                    origin.set(axis, str(int(origin.get(axis))+round(value*int(doc['pageSize'][dimension]))))
            control_parts = dict(parts)
            control_parts[slide] = X.tostring(control)
            control_bytes = motion.archive(control_parts)
            qc = copy.deepcopy(q['page'])
            qc['page']['expectedSourceSha256'] = f.sha(control_bytes)
            _, _, expected_pixels = f.native(args.worker, qc, control_bytes, b'', '--pptx-resource-page')
            delta = np.abs(np.frombuffer(rendered['source'], dtype=np.uint8).astype(np.int16)-np.frombuffer(expected_pixels, dtype=np.uint8))
            assert int(delta.max()) <= 2 and int(delta.sum()) <= 1000, (name, ms, int(delta.max()), int(delta.sum()))
            controls.append(dict(name=f'{name}-{index}', maxChannelDifference=int(delta.max()), totalChannelDifference=int(delta.sum()), source=f.put(out/f'{name}-{index}.control.pptx', control_bytes), request=f.put(out/f'{name}-{index}.control.json', qc)))
    for kind in cases:
        f.put(out/(kind+'.json'), dict(cases=cases[kind]))
    for item in inputs.values():
        f.load(item)
    report = dict(status='passed', nativeFrames=sum(map(len, cases.values())), checks=checks, staticControls=controls, xsdParts=xsd_parts, inputs=inputs)
    f.put(out/'report.json', report)
    print(json.dumps(dict(status='passed', frames=report['nativeFrames'], controls=len(controls), xsdParts=len(xsd_parts), maxPositionError=max(c['maxNormalizedPositionError'] for c in checks))))


if __name__ == '__main__':
    main()
