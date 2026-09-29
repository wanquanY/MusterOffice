"""Owned editable PPTX seek corpus, exact clocks and author/source raster parity.

Independent equation assertions complement shared-kernel native/WASM replay.
This is not a recording of WPS or PowerPoint animation playback.
"""
import argparse
import copy
from fractions import Fraction
import importlib.util
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as X
import zipfile

spec = importlib.util.spec_from_file_location('fixture', Path(__file__).with_name('scale-playback-fixtures.py'))
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['output', 'cli', 'worker', 'author-input']:
        parser.add_argument('--'+key, type=Path, required=True)
    args = parser.parse_args()
    out = args.output
    out.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): f.entry(p) for p in [args.cli, args.worker, args.author_input]}
    records = {'author': [], 'source': []}
    checks = []
    empty = f.put(out/'empty.bin', b'')
    viewport = dict(width=800, height=450, origin=dict(x='0', y='0'), scale=dict(numerator=1, denominator=10000), coordinateTolerance='16777216', background=[255]*4)
    condition = lambda direction: dict(kind='navigation', direction=direction, target=None, delay=f.time(0))
    for name in ['finite', 'infinite-concurrent', 'infinite-exclusive', 'nested-dependency', 'cached-end']:
        authored = json.loads(args.author_input.read_text())
        timeline = authored['document']['timelines']['slide:1']
        timeline['format'] = 'musteroffice.timeline/0.2-draft'
        timeline['nodes'] = timeline['nodes'][:4 if name == 'nested-dependency' else 3]
        nodes = timeline['nodes']
        for node in nodes:
            node['start'] = dict(kind='never')
        ids = [n['id'] for n in nodes]
        main = dict(id='main', kind='sequence', start=dict(kind='at', offset=f.time(0)), duration=dict(kind='indefinite'), fill='hold', children=ids[:3], navigation=dict(concurrent=name != 'infinite-exclusive', nextAction='seek', previousAction='none', nextConditions=[condition('next')], previousConditions=[condition('previous')]))
        timeline['tree'] = dict(roots=['main'], containers=[main])
        if name.startswith('infinite'):
            nodes[0]['repeatMilli'] = 'indefinite'
        if name in ['nested-dependency', 'cached-end']:
            main['children'] = ['group', ids[2]]
            timeline['tree']['containers'].append(dict(id='group', kind='parallel', start=dict(kind='never'), duration=dict(kind='indefinite'), fill='hold', children=ids[:2]))
            nodes[0]['start'] = dict(kind='at', offset=f.time(0))
            if name == 'nested-dependency':
                nodes[1]['start'] = dict(kind='after', node=ids[0], event='end', delay=f.time(1,5))
                nodes[3]['start'] = dict(kind='after', node=ids[1], event='end', delay=f.time(1,5))
                timeline['tree']['roots'].append(ids[3])
            else:
                nodes[1]['start'] = dict(kind='at', offset=f.time(1,2))
                nodes[1]['duration'] = f.time(2)
                nodes[1]['endConditions'] = [dict(kind='after', node=ids[0], event='begin', delay=f.time(7,10))]
        exported = f.put(out/(name+'.export.json'), authored)
        pptx = out/(name+'.pptx')
        result = subprocess.run([str(args.cli), 'pptx-export', exported['path'], empty['path'], str(pptx)], capture_output=True, env={}, timeout=60)
        assert result.returncode == 0 and not result.stderr, result.stderr
        f.put(out/(name+'.export-response.json'), result.stdout)
        result = subprocess.run([str(args.cli)], input=json.dumps(dict(operation='initialize', document=authored['document'])).encode(), capture_output=True, env={}, timeout=60)
        assert result.returncode == 0 and not result.stderr
        snapshot = json.loads(result.stdout)['snapshot']
        source = f.entry(pptx)
        a = dict(playback=dict(snapshot=snapshot, slide='slide:1', binding=dict(session=name, revision=snapshot['revision'], generation='1'), at=None, history=None), viewport=viewport, defaults=dict(themeColors={}, pageBackground=dict(red=255,green=255,blue=255,alpha=255)))
        page = dict(profile='drawingml-resource-page-q32-v1-draft', page=dict(expectedSourceSha256=source['sha256'], slide='/ppt/slides/slide1.xml', profile='drawingml-static-solid-page-v1-draft', colorContext=dict(systemColors={}, placeholder=None), viewport=viewport), imageSource='embeddedSnapshot', sampling='nearest', fonts=None)
        b = dict(page=page, sample=dict(binding=dict(session=name, revision=source['sha256'], generation='1'), at=None, history=None))
        for sample in [a['playback'], b['sample']]:
            sample['history'] = dict(binding=sample['binding'], through=f.time(10), events=[dict(sequence=i+1, generation='1', at=f.time(ms,1000), event=dict(kind='navigation', direction=direction, target=None)) for i,(ms,direction) in enumerate([(100,'next'), (300,'next'), (500,'previous'), (700,'next')])])
        with zipfile.ZipFile(pptx) as package:
            xml = package.read('ppt/slides/slide1.xml')
            assert b'nextAc="seek"' in xml
            slide = X.fromstring(xml)
        objects = {n.get('name'): 'sp.'+n.get('id') for n in slide.findall('.//p:cNvPr', f.NS) if n.get('name')}
        for index, ms in enumerate([0,100,200,300,350,400,500,550,700,750,1000,2500,400]):
            a['playback']['at'] = b['sample']['at'] = f.time(ms,1000)
            states, pixels = [], []
            for kind, request in [('author',a), ('source',b)]:
                response, raw, rgba = f.native(args.worker, request, f.load(source) if kind == 'source' else None, b'' if kind == 'source' else None)
                stem = out/f'{name}-{kind}-{index}'
                record = dict(name=stem.name, request=f.put(stem.with_suffix('.request.json'), request), response=f.put(stem.with_suffix('.response.json'), raw), pixels=f.put(stem.with_suffix('.rgba'), rgba))
                if kind == 'source': record.update(source=source, fonts=empty)
                records[kind].append(record)
                states.append(response['info']['frame']['state'] if kind == 'author' else response['info']['playback']['evaluated']['state'])
                pixels.append(rgba)
            sa,sb = states
            assert pixels[0] == pixels[1], (name,ms)
            assert {objects[k]:v for k,v in sa.get('scales',{}).items()} == sb.get('scales',{})
            for group in ['nodes', 'containers']:
                for na,nb in zip(sa[group],sb[group],strict=True):
                    assert {k:v for k,v in na.items() if k!='node'} == {k:v for k,v in nb.items() if k!='node'}, (name,ms,na,nb)
            assert [s['position'] for s in sa['sequences']] == [s['position'] for s in sb['sequences']]
            if ms == 400:
                expected = {'shape:0': 110000 if name=='infinite-concurrent' else 200000}
                if name in ['nested-dependency','cached-end']:
                    expected.update({'shape:1':110000 if name=='cached-end' else 200000, 'shape:2':110000})
                    assert 'shape:3' not in sa.get('scales',{})
                else: expected['shape:1'] = 110000
                for key, value in expected.items():
                    assert sa['scales'][key] == dict(x=f.exact(Fraction(value)),y=f.exact(Fraction(value))), (name,sa)
            checks.append(dict(name=f'{name}-{index}', time=f.time(ms,1000), pixelSha256=f.sha(pixels[0])))
    for kind,cases in records.items(): f.put(out/(kind+'.json'),dict(cases=cases))
    for item in inputs.values(): f.load(item)
    report = dict(status='passed', frameCount=sum(map(len,records.values())), authorSourcePairs=len(checks), inputs=inputs, checks=checks, scope=__doc__)
    f.put(out/'report.json',report)
    print(json.dumps({k:report[k] for k in ['status','frameCount','authorSourcePairs']}))


if __name__ == '__main__':
    main()
