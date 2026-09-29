"""Export native groups and compare their editable playback to author rendering.

Owned inputs only. Requires explicit matching CLI/worker paths and a request;
keeps original declarations, checks native structure and every rendered pixel.
Produces manifests consumed by the independent SDK/browser replay harnesses.
"""
import argparse
import copy
import importlib.util
import json
import subprocess
import zipfile
from pathlib import Path
import xml.etree.ElementTree as X

_spec = importlib.util.spec_from_file_location(
    'scale_fixtures', Path(__file__).with_name('scale-playback-fixtures.py'))
fixtures = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(fixtures)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['output', 'worker', 'cli', 'request', 'page']:
        parser.add_argument('--'+name, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    inputs = [fixtures.entry(p) for p in [args.worker, args.cli, args.request, args.page]]
    request = json.loads(fixtures.load(inputs[2]))
    page = json.loads(fixtures.load(inputs[3]))
    assert not request['document']['resources'] and not request['resourceBindings']
    empty = fixtures.put(args.output/'empty.bin', b'')
    cases = dict(author=[], source=[])
    exports = []
    times = [(0, 1), (1, 7), (1, 2), (1, 1), (2, 1), (7, 3), (1, 7), (0, 1)]
    for name in ['original', 'child-stroke', 'nested', 'nested-child-stroke']:
        export = copy.deepcopy(request)
        doc = export['document']
        if 'child-stroke' in name:
            doc['objects']['shape:1']['appearance']['stroke'] = dict(kind='value', value=dict(
                kind='solid', width='50000', cap='round', join=dict(kind='bevel'),
                color=dict(kind='srgb', rgba=dict(red=28, green=155, blue=63, alpha=255))))
        if name.startswith('nested'):
            group = copy.deepcopy(doc['objects']['group:1'])
            group['id'] = 'group:outer'
            group['content']['children'] = ['group:1']
            group['content']['viewport'] = dict(width='3200000', height='2400000')
            group['transform'] = dict(origin=dict(x='150000', y='180000'),
                size=dict(width='2800000', height='2000000'), rotation=2700000,
                flipHorizontal=False, flipVertical=True)
            doc['objects'][group['id']] = group
            doc['objects']['group:1']['parent'] = dict(kind='group', id=group['id'])
            doc['slides']['slide:1']['objects'] = [group['id']]
        export_input = fixtures.put(args.output/(name+'.export.json'), export)
        output = args.output/(name+'.pptx')
        result = subprocess.run([str(args.cli), 'pptx-export', export_input['path'],
            empty['path'], str(output)], capture_output=True, timeout=60, env={})
        assert result.returncode == 0 and not result.stderr, result.stderr
        assert json.loads(result.stdout)['status'] == 'inspected'
        source = fixtures.entry(output)
        with zipfile.ZipFile(output) as z:
            xml = X.fromstring(z.read('ppt/slides/slide1.xml'))
            # There must be real group and shape nodes, never a flattened image.
            assert not xml.findall('.//p:pic', fixtures.NS)
            groups = xml.findall('.//p:grpSp', fixtures.NS)
            assert len(groups) == (2 if name.startswith('nested') else 1)
            for group in groups:
                properties = group.find('p:grpSpPr', fixtures.NS)
                assert properties is not None and properties.find('a:ln', fixtures.NS) is None
            assert len(xml.findall('.//p:sp', fixtures.NS)) == 2
            assert len(xml.findall('.//p:animRot', fixtures.NS)) == 2
            assert len(xml.findall('.//p:animScale', fixtures.NS)) == 1
            native_ids = {n.get('name'): 'sp.'+n.get('id')
                for n in xml.findall('.//p:cNvPr', fixtures.NS) if n.get('name')}
            assert set(native_ids) == set(doc['objects'])
        initialized = subprocess.run([str(args.cli)], input=json.dumps(dict(
            operation='initialize', document=doc)).encode(), capture_output=True, timeout=60, env={})
        assert initialized.returncode == 0 and not initialized.stderr, initialized.stderr
        snapshot = json.loads(initialized.stdout)['snapshot']
        binding = dict(session='group-'+name, revision=snapshot['revision'], generation='1')
        for i, (n, d) in enumerate(times):
            at = fixtures.time(n, d)
            author = dict(playback=dict(snapshot=snapshot, slide='slide:1', binding=binding,
                at=at, history=None), viewport=page['viewport'], defaults=page['defaults'])
            native = dict(page=dict(profile='drawingml-resource-page-q32-v1-draft', page=dict(
                expectedSourceSha256=source['sha256'], slide='/ppt/slides/slide1.xml',
                profile='drawingml-static-solid-page-v1-draft', colorContext=dict(
                    systemColors={}, placeholder=None), viewport=page['viewport']),
                imageSource='embeddedSnapshot', sampling='nearest', fonts=None),
                sample=dict(binding={**binding, 'revision': source['sha256']}, at=at, history=None))
            outputs = {}
            for kind, query in [('author', author), ('source', native)]:
                response, metadata, pixels = fixtures.native(args.worker, query,
                    None if kind == 'author' else fixtures.load(source),
                    None if kind == 'author' else fixtures.load(empty))
                stem = args.output/(name+'-'+kind+'-'+str(i))
                case = dict(name=name+'-'+str(i),
                    request=fixtures.put(stem.with_suffix('.request.json'), query),
                    response=fixtures.put(stem.with_suffix('.response.json'), metadata),
                    pixels=fixtures.put(stem.with_suffix('.rgba'), pixels))
                if kind == 'source':
                    case.update(source=source, fonts=empty)
                cases[kind].append(case)
                state = response['info']['frame']['state'] if kind == 'author' else response['info']['playback']['evaluated']['state']
                outputs[kind] = (state, pixels)
            a, s = outputs['author'], outputs['source']
            assert a[1] == s[1], (name, at, 'complete pixel bytes differ')
            for channel in ['rotations', 'scales']:
                assert {native_ids[k]: v for k, v in a[0][channel].items()} == s[0][channel]
        exports.append(dict(name=name, request=export_input, output=source, objectIds=native_ids))
    for kind in cases:
        fixtures.put(args.output/(kind+'.json'), dict(format='musteroffice.group-export-playback/1', cases=cases[kind]))
    for record in inputs:
        fixtures.load(record)
    report = dict(status='passed', exports=exports, inputs=inputs,
        comparedAuthorSourceFrames=len(cases['author']), nativeFrames=sum(map(len, cases.values())),
        scope='Editable group structure and author/source parity; no Office/WPS acceptance')
    fixtures.put(args.output/'report.json', report)
    print(json.dumps({k: report[k] for k in ['status', 'comparedAuthorSourceFrames', 'nativeFrames']}))


if __name__ == '__main__':
    main()
