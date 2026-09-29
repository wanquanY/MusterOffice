"""Exercise native sequence roundtrip and an owned WPS-saved scale document.

Produces pinned source/author manifests for public SDK replay. Native/WASM
agreement is a shared-kernel check, not an independent Office reference.
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
    for name in ['output', 'cli', 'worker', 'author-input', 'wps-source']:
        parser.add_argument('--'+name, type=Path, required=True)
    args = parser.parse_args()
    out = args.output
    out.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): f.entry(p) for p in [args.cli, args.worker, args.author_input, args.wps_source]}
    records = {'author': [], 'source': []}
    checks = []
    empty = f.put(out/'empty.bin', b'')
    viewport = dict(width=800, height=450, origin=dict(x='0', y='0'), scale=dict(numerator=1, denominator=10000), coordinateTolerance='16777216', background=[255]*4)

    def source_request(source, session):
        binding = dict(session=session, revision=source['sha256'], generation='1')
        page = dict(profile='drawingml-resource-page-q32-v1-draft', page=dict(expectedSourceSha256=source['sha256'], slide='/ppt/slides/slide1.xml', profile='drawingml-static-solid-page-v1-draft', colorContext=dict(systemColors={}, placeholder=None), viewport=viewport), imageSource='embeddedSnapshot', sampling='nearest', fonts=None)
        return dict(page=page, sample=dict(binding=binding, at=None, history=None))

    def render(kind, name, request, source=None):
        response, raw, pixels = f.native(args.worker, request, f.load(source) if source else None, b'' if source else None)
        stem = out/name
        record = dict(name=name, request=f.put(stem.with_suffix('.request.json'), request), response=f.put(stem.with_suffix('.response.json'), raw), pixels=f.put(stem.with_suffix('.rgba'), pixels))
        if source:
            record.update(source=source, fonts=empty)
        records[kind].append(record)
        state = response['info']['frame']['state'] if kind == 'author' else response['info']['playback']['evaluated']['state']
        return state, pixels

    source = f.entry(args.wps_source)
    request = source_request(source, 'wps-saved-scale')
    request['sample']['history'] = dict(binding=request['sample']['binding'], through=f.time(10), events=[])
    with zipfile.ZipFile(source['path']) as package:
        slide = X.fromstring(package.read('ppt/slides/slide1.xml'))
    shapes = []
    for shape in slide.findall('p:cSld/p:spTree/p:sp', f.NS):
        props = shape.find('p:spPr', f.NS)
        transform = props.find('a:xfrm', f.NS)
        offset, extent = transform.find('a:off', f.NS), transform.find('a:ext', f.NS)
        color = props.find('a:solidFill/a:srgbClr', f.NS).get('val')
        shapes.append((bytes.fromhex(color)+b'\xff', *(int(offset.get(k)) for k in ['x','y']), *(int(extent.get(k)) for k in ['cx','cy'])))
    assert len(shapes) == 6
    for index, (n, d) in enumerate([(0,1), (1,3), (1,2), (1,1), (5,1), (1,2)]):
        at = Fraction(n,d)
        request['sample']['at'] = f.time(n,d)
        state, pixels = render('source', f'wps-{index}', request, source)
        factor = 1+min(at, 1)
        expected = f.exact(100000*factor)
        assert len(state['scales']) == 6 and len(state['containers']) == 9
        assert all(v == dict(x=expected, y=expected) for v in state['scales'].values())
        bounds = []
        for color, x, y, width, height in shapes:
            matches = [i for i in range(800*450) if pixels[4*i:4*i+4] == color]
            assert matches
            actual = [min(i%800 for i in matches), min(i//800 for i in matches), max(i%800 for i in matches)+1, max(i//800 for i in matches)+1]
            cx, cy = Fraction(2*x+width, 20000), Fraction(2*y+height, 20000)
            hw, hh = Fraction(width,20000)*factor, Fraction(height,20000)*factor
            geometric = [cx-hw, cy-hh, cx+hw, cy+hh]
            assert all(abs(a-b) <= 1 for a,b in zip(actual, geometric)), (actual, geometric)
            bounds.append(actual)
        checks.append(dict(name=f'wps-{index}', time=f.time(n,d), bounds=bounds, pixelSha256=f.sha(pixels)))

    export_input = json.loads(args.author_input.read_text())
    for concurrent in [False, True]:
        name = 'concurrent' if concurrent else 'exclusive'
        authored = copy.deepcopy(export_input)
        timeline = authored['document']['timelines']['slide:1']
        timeline['format'] = 'musteroffice.timeline/0.2-draft'
        timeline['nodes'] = timeline['nodes'][:3]
        for node in timeline['nodes']:
            node['start'] = dict(kind='never')
        condition = lambda direction: dict(kind='navigation', direction=direction, target=None, delay=f.time(0))
        timeline['tree'] = dict(roots=['main'], containers=[dict(id='main', kind='sequence', start=dict(kind='at', offset=f.time(0)), duration=dict(kind='indefinite'), fill='hold', children=[n['id'] for n in timeline['nodes']], navigation=dict(concurrent=concurrent, nextAction='none', previousAction='skipTimed', nextConditions=[condition('next')], previousConditions=[condition('previous')]))])
        export_request = f.put(out/(name+'.export.json'), authored)
        pptx = out/(name+'.pptx')
        result = subprocess.run([str(args.cli), 'pptx-export', export_request['path'], empty['path'], str(pptx)], capture_output=True, env={}, timeout=60)
        assert result.returncode == 0 and not result.stderr, result.stderr
        f.put(out/(name+'.export-response.json'), result.stdout)
        source = f.entry(pptx)
        result = subprocess.run([str(args.cli)], input=json.dumps(dict(operation='initialize', document=authored['document'])).encode(), capture_output=True, env={}, timeout=60)
        assert result.returncode == 0 and not result.stderr
        initialized = json.loads(result.stdout)
        assert initialized['status'] == 'initialized', initialized
        snapshot = initialized['snapshot']
        a = dict(playback=dict(snapshot=snapshot, slide='slide:1', binding=dict(session=name, revision=snapshot['revision'], generation='1'), at=None, history=None), viewport=viewport, defaults=dict(themeColors={}, pageBackground=dict(red=255,green=255,blue=255,alpha=255)))
        b = source_request(source, name)
        with zipfile.ZipFile(pptx) as package:
            slide = X.fromstring(package.read('ppt/slides/slide1.xml'))
        objects = {n.get('name'): 'sp.'+n.get('id') for n in slide.findall('.//p:cNvPr', f.NS) if n.get('name')}
        for sample in [a['playback'], b['sample']]:
            sample['history'] = dict(binding=sample['binding'], through=f.time(10), events=[dict(sequence=i+1, generation='1', at=f.time(ms,1000), event=dict(kind='navigation', direction=direction, target=None)) for i,(ms,direction) in enumerate([(100,'next'), (400,'next'), (700,'previous'), (900,'next')])])
        for index, ms in enumerate([0,100,200,399,400,500,700,800,900,1000,1300,2000,400]):
            a['playback']['at'] = b['sample']['at'] = f.time(ms,1000)
            sa, pa = render('author', f'{name}-author-{index}', a)
            sb, pb = render('source', f'{name}-source-{index}', b, source)
            assert pa == pb, (name, ms)
            for prop in ['rotations', 'scales']:
                assert {objects[k]:v for k,v in sa.get(prop,{}).items()} == sb.get(prop,{})
            for group in ['nodes', 'containers']:
                for na, nb in zip(sa[group], sb[group], strict=True):
                    assert {k:v for k,v in na.items() if k!='node'} == {k:v for k,v in nb.items() if k!='node'}
            assert [s['position'] for s in sa['sequences']] == [s['position'] for s in sb['sequences']]
            checks.append(dict(name=f'{name}-{index}', time=f.time(ms,1000), pixelSha256=f.sha(pa)))
    # An oversized opaque shape on a transparent fractional page must remain
    # clipped to the document, including half-covered final rows/columns.
    edge = copy.deepcopy(export_input)
    doc = edge['document']
    doc['pageSize'] = dict(width='7995000', height='4495000')
    doc['timelines'] = {}
    doc['slides']['slide:1']['objects'] = ['shape:0']
    doc['objects'] = {'shape:0': doc['objects']['shape:0']}
    doc['objects']['shape:0']['transform'].update(origin=dict(x='-10000', y='-10000'), size=dict(width='9000000', height='5000000'))
    ep = f.put(out/'fractional.export.json', edge)
    pptx = out/'fractional.pptx'
    exported = subprocess.run([str(args.cli), 'pptx-export', ep['path'], empty['path'], str(pptx)], capture_output=True, env={}, timeout=60)
    assert exported.returncode == 0 and not exported.stderr
    init = subprocess.run([str(args.cli)], input=json.dumps(dict(operation='initialize',document=doc)).encode(), capture_output=True, env={}, timeout=60)
    assert init.returncode == 0 and not init.stderr
    snapshot = json.loads(init.stdout)['snapshot']
    viewport['background'] = [0]*4
    a = dict(playback=dict(snapshot=snapshot, slide='slide:1', binding=dict(session='fractional', revision=snapshot['revision'], generation='1'), at=f.time(0), history=None), viewport=viewport, defaults=dict(themeColors={}, pageBackground=dict(red=0,green=0,blue=0,alpha=0)))
    source = f.entry(pptx)
    b = source_request(source, 'fractional')
    b['sample']['at'] = f.time(0)
    _, pa = render('author', 'fractional-author', a)
    _, pb = render('source', 'fractional-source', b, source)
    assert pa == pb
    pixel = lambda x,y: list(pa[4*(y*800+x):4*(y*800+x)+4])
    assert pixel(100,100) == [220,40,30,255]
    for x,y,expected in [(799,100,128), (100,449,128), (799,449,64)]:
        rgba = pixel(x,y)
        assert abs(rgba[3]-expected) <= 1, (x,y,rgba)
        assert all(abs(channel - base*rgba[3]/255) <= 1 for channel,base in zip(rgba[:3], [220,40,30]))
    checks.append(dict(name='fractional-boundary', edge=pixel(799,100), corner=pixel(799,449), pixelSha256=f.sha(pa)))
    for kind, cases in records.items():
        f.put(out/(kind+'.json'), dict(cases=cases))
    for record in inputs.values():
        f.load(record)
    report = dict(status='passed', inputs=inputs, frameCount=sum(map(len, records.values())), wpsSourceFrames=6, authorSourcePairs=27, checks=checks, scope='Actual WPS-saved timing and owned native navigation exports. Geometric bounds, exact transform values and fractional page clipping; no new Office playback observation or full compatibility acceptance.')
    f.put(out/'report.json', report)
    print(json.dumps({key: report[key] for key in ['status','frameCount','wpsSourceFrames','authorSourcePairs']}))


if __name__ == '__main__':
    main()
