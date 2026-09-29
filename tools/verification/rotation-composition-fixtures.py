"""Owned rotation forms checked against independent angles and static geometry.

Retained source data is read from complete PPTX files. Expected angles are
computed here using Fraction, independently of the Rust sampler and writer.
"""
import argparse
import copy
from fractions import Fraction
import importlib.util
import io
import json
from pathlib import Path
import xml.etree.ElementTree as X
import zipfile
from lxml import etree

spec = importlib.util.spec_from_file_location('native', Path(__file__).with_name('scale-playback-fixtures.py'))
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)


def archive(parts):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, 'w', compression=zipfile.ZIP_DEFLATED) as z:
        for name, data in parts.items():
            z.writestr(name, data)
    return stream.getvalue()


def angle(name, at):
    progress = lambda elapsed: min(max(elapsed, 0), 1)
    first, second = 30 * progress(at), 70 * progress(at - 3)
    if name == 'rotation-by':
        return 20 + first + second
    if name == 'rotation-layout':
        return 20 + (first if at < 3 else second)
    if name == 'rotation-absolute':
        return first if at < 3 else second
    if name == 'rotation-overlap':
        return 20 + (first if at < 1 else 0) + 70 * progress(at - Fraction(1, 2))
    raise ValueError(name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['output', 'sources', 'worker', 'template', 'schema']:
        parser.add_argument('--' + key, type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    inputs = {}

    def tracked(path):
        record = f.entry(path.resolve())
        inputs[record['path']] = record
        return f.load(record)

    for path in [args.worker, Path(__file__), Path(f.__file__)]:
        tracked(path)
    for path in args.schema.parent.glob('*.xsd'):
        tracked(path)
    schema = etree.XMLSchema(etree.parse(str(args.schema)))
    template = json.loads(tracked(args.template))
    times = [Fraction(ms, 1000) for ms in [0, 250, 500, 750, 1000, 1250, 1500, 2500, 3000, 3250, 3500, 3750, 4000, 4500, 5500, 250]]
    times += [Fraction(1, 7), 3 + Fraction(1, 7)]
    records, checks, controls, schema_parts = [], [], [], []
    fonts = f.put(out / 'empty.bin', b'')
    for name in ['rotation-by', 'rotation-layout', 'rotation-absolute', 'rotation-overlap']:
        raw = tracked(args.sources / (name + '.pptx'))
        source = f.put(out / (name + '.pptx'), raw)
        with zipfile.ZipFile(io.BytesIO(raw)) as z:
            parts = {p: z.read(p) for p in z.namelist()}
        for part, data in parts.items():
            if not part.endswith('.xml'):
                continue
            root = etree.fromstring(data)
            if root.tag.startswith('{' + f.P + '}'):
                schema.assertValid(root)
                schema_parts.append(dict(source=source, part=part))
        slide = 'ppt/slides/slide1.xml'
        tree = X.fromstring(parts[slide])
        for timing in tree.findall('p:timing', f.NS):
            tree.remove(timing)
        targets = {}
        for obj in tree.findall('p:cSld/p:spTree/p:sp', f.NS):
            native_id = int(obj.find('p:nvSpPr/p:cNvPr', f.NS).get('id'))
            base = int(obj.find('p:spPr/a:xfrm', f.NS).get('rot', '0'))
            assert base == 20 * 60000
            targets[native_id] = base
        assert len(targets) == 6
        binding = dict(session=name, revision=source['sha256'], generation='1')
        q = copy.deepcopy(template)
        q['page']['page']['expectedSourceSha256'] = source['sha256']
        q['sample']['binding'] = binding
        q['sample']['history'] = dict(binding=binding, through=f.time(10), events=[dict(
            at=f.time(3), generation='1', sequence=1,
            event=dict(kind='navigation', direction='next', target=None))])
        for index, at in enumerate(times):
            q['sample']['at'] = f.time(at.numerator, at.denominator)
            response, metadata, pixels = f.native(args.worker, q, raw, b'')
            absolute = angle(name, at) * 60000
            expected = {'sp.' + str(target): dict(**f.exact(absolute - base), basis='layout') for target, base in targets.items()}
            state = response['info']['playback']['evaluated']['state']
            assert state['rotations'] == expected, (name, str(at), state['rotations'], expected)
            stem = name + '-' + str(index)
            records.append(dict(name=stem, request=f.put(out / (stem + '.request.json'), q),
                                response=f.put(out / (stem + '.response.json'), metadata),
                                pixels=f.put(out / (stem + '.rgba'), pixels), source=source, fonts=fonts))
            checks.append(dict(name=stem, time=str(at), expectedAbsoluteAngle=f.exact(absolute), expectedRotations=expected))
            if absolute.denominator == 1:
                control = copy.deepcopy(tree)
                for obj in control.findall('p:cSld/p:spTree/p:sp', f.NS):
                    obj.find('p:spPr/a:xfrm', f.NS).set('rot', str(absolute.numerator))
                static_parts = dict(parts)
                static_parts[slide] = X.tostring(control)
                static = archive(static_parts)
                qc = copy.deepcopy(q['page'])
                qc['page']['expectedSourceSha256'] = f.sha(static)
                _, _, expected_pixels = f.native(args.worker, qc, static, b'', '--pptx-resource-page')
                assert pixels == expected_pixels, stem + ' static-angle oracle'
                controls.append(dict(name=stem, source=f.put(out / (stem + '.control.pptx'), static),
                                     request=f.put(out / (stem + '.control.json'), qc), pixelSha256=f.sha(expected_pixels)))
    f.put(out / 'source.json', dict(cases=records))
    f.put(out / 'author.json', dict(cases=[]))
    for record in inputs.values():
        f.load(record)
    f.put(out / 'report.json', dict(status='passed', inputs=inputs, frames=len(records), checks=checks, staticControls=controls, xsdParts=schema_parts))
    print(json.dumps(dict(status='passed', frames=len(records), staticControls=len(controls), xsdParts=len(schema_parts))))


if __name__ == '__main__':
    main()
