"""Owned transform fixtures through the actual native worker, ready for SDK replay.

Inputs are pinned prior fixtures. No Office/WPS acceptance is inferred. Every
source object keeps its original text, geometry and resources while sampling.
"""
import argparse
import copy
import hashlib
import io
import json
import subprocess
import zipfile
from fractions import Fraction
from pathlib import Path
import xml.etree.ElementTree as X

P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
NS = {'p': P, 'a': A}

def sha(b):
    return hashlib.sha256(b).hexdigest()

def entry(p):
    p = Path(p)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=sha(b))

def load(r):
    assert entry(r['path']) == r, r['path']
    return Path(r['path']).read_bytes()

def put(p, value):
    if not isinstance(value, bytes):
        value = json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()
    with p.open('xb') as stream:
        stream.write(value)
    return entry(p)

def time(n, d=1):
    return dict(ticks=str(n), timescale=d)

def exact(v):
    return dict(numerator=str(v.numerator), denominator=str(v.denominator))

def sub(parent, name, **attrs):
    return X.SubElement(parent, '{'+P+'}'+name, {k: str(v) for k, v in attrs.items()})

def native(worker, q, source=None, fonts=None, mode=None):
    payload = json.dumps(q, separators=(',', ':')).encode()
    blocks = [payload] if source is None else [payload, source, fonts]
    header = b''.join(len(b).to_bytes(4, 'little') for b in blocks)
    args = [str(worker), mode or ('--playback-page' if source is None else '--pptx-playback-page')]
    result = subprocess.run(args, input=header+b''.join(blocks), env={}, capture_output=True, timeout=60)
    assert result.returncode == 0 and not result.stderr, result.stderr.decode(errors='replace')
    ml, pl = [int.from_bytes(result.stdout[i:i+4], 'little') for i in [0, 4]]
    assert len(result.stdout) == 8+ml+pl
    metadata, pixels = result.stdout[8:8+ml], result.stdout[8+ml:]
    response = json.loads(metadata)
    assert response['status'] == 'rendered', response
    return response, metadata, pixels

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['output', 'worker', 'cli', 'author', 'source']:
        parser.add_argument('--'+key, type=Path, required=True)
    args = parser.parse_args()
    root = args.output
    root.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): entry(p) for p in [args.worker, args.cli, args.author, args.source]}
    def record_input(p):
        inputs[p['path']] = p
        return load(p)
    def run(kind, name, q, source=None, fonts=None):
        response, metadata, pixels = native(args.worker, q, None if source is None else record_input(source), None if fonts is None else record_input(fonts))
        p = root/name
        c = dict(name=name, request=put(p.with_suffix('.request.json'), q), response=put(p.with_suffix('.response.json'), metadata), pixels=put(p.with_suffix('.rgba'), pixels))
        if source is not None:
            c.update(source=source, fonts=fonts)
        state = response['info']['frame']['state'] if kind == 'author' else response['info']['playback']['evaluated']['state']
        assert state['profile'] == 'musteroffice.transform-frame/0.1-draft'
        return c, state, pixels
    page = json.loads(args.author.read_text())
    records = dict(author=[], source=[])
    nodes = page['page']['document']['timelines'][page['page']['slide']]['nodes']
    for target in ['group:1', 'shape:1']:
        n = copy.deepcopy(nodes[0]); n['id'] = 'scale:'+target
        n['effect'] = dict(kind='scale', target=target, **{'from': dict(x=0, y=0), 'to': dict(x=200000, y=100000)})
        nodes.append(n)
    init = subprocess.run([str(args.cli)], input=json.dumps(dict(operation='initialize', document=page['page']['document'])).encode(), capture_output=True, timeout=60, env={})
    assert init.returncode == 0 and not init.stderr, init.stderr
    snapshot = json.loads(init.stdout)['snapshot']
    binding = dict(session='scale-author', revision=snapshot['revision'], generation='3')
    times = [(0, 1), (1, 7), (1, 2), (1, 1), (2, 1), (1, 7)]
    for i, (n, d) in enumerate(times):
        q = dict(playback=dict(snapshot=snapshot, slide=page['page']['slide'], binding=binding, at=time(n, d), history=None), viewport=page['viewport'], defaults=page['defaults'])
        c, state, _ = run('author', 'author-'+str(i), q)
        expected = dict(x=exact(Fraction(100000*n, d)), y=exact(Fraction(50000*n, d)))
        assert state['scales'] == {target: expected for target in ['group:1', 'shape:1']}
        records['author'].append(c)
    previous = json.loads(args.source.read_text())
    for name in ['image-text', 'group-image', 'circle']:
        case = next(c for c in previous['cases'] if c['name'] == name+'-0')
        raw = record_input(case['source']); fonts = case['fonts']; record_input(fonts)
        q = json.loads(record_input(case['request']))
        with zipfile.ZipFile(io.BytesIO(raw)) as z:
            parts = {n: z.read(n) for n in z.namelist()}
        slide = q['page']['page']['slide'].lstrip('/')
        tree = X.fromstring(parts[slide])
        for old in tree.findall('p:timing', NS):
            tree.remove(old)
        objects = tree.findall('.//p:sp', NS)+tree.findall('.//p:pic', NS)+tree.findall('.//p:grpSp', NS)
        ids = [int(o.find('./*/p:cNvPr', NS).get('id')) for o in objects]
        assert ids
        timing = sub(tree, 'timing'); listing = sub(sub(sub(sub(timing, 'tnLst'), 'par'), 'cTn', id=1, dur='indefinite', restart='never', nodeType='tmRoot'), 'childTnLst')
        for i, target in enumerate(ids):
            anim = sub(listing, 'animScale')
            behavior = sub(anim, 'cBhvr', additive='repl', accumulate='none', xfrmType='pt')
            common = sub(behavior, 'cTn', id=i+2, dur=2000, repeatCount=1000, restart='never', fill='freeze')
            sub(sub(common, 'stCondLst'), 'cond', delay=0)
            sub(sub(behavior, 'tgtEl'), 'spTgt', spid=target)
            names = sub(behavior, 'attrNameLst')
            for axis in ['ScaleX', 'ScaleY']:
                sub(names, 'attrName').text = axis
            sub(anim, 'from', x=0, y=0); sub(anim, 'to', x=200000, y=100000)
        parts[slide] = X.tostring(tree)
        path = root/(name+'.pptx')
        with zipfile.ZipFile(path, 'x') as z:
            for part, data in parts.items():
                info = zipfile.ZipInfo(part, (2026, 9, 27, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, data)
        source = entry(path)
        q['page']['page']['expectedSourceSha256'] = source['sha256']
        q['sample']['binding']['revision'] = source['sha256']
        for i, (n, d) in enumerate(times):
            q['sample']['at'] = time(n, d)
            c, state, pixels = run('source', name+'-'+str(i), q, source, fonts)
            expected = dict(x=exact(Fraction(100000*n, d)), y=exact(Fraction(50000*n, d)))
            assert state['scales'] == {'sp.'+str(target): expected for target in ids}
            records['source'].append(c)
            if n == 0:
                control = copy.deepcopy(tree); control.remove(control.find('p:timing', NS))
                objects_root = control.find('p:cSld/p:spTree', NS)
                for o in list(objects_root):
                    if o.tag in ['{'+P+'}'+kind for kind in ['sp', 'pic', 'grpSp']]:
                        objects_root.remove(o)
                control_parts = dict(parts); control_parts[slide] = X.tostring(control)
                stream = io.BytesIO()
                with zipfile.ZipFile(stream, 'w') as z:
                    for part, data in control_parts.items():
                        z.writestr(part, data)
                source_control = stream.getvalue(); query = copy.deepcopy(q['page'])
                query['page']['expectedSourceSha256'] = sha(source_control)
                _, _, expected_pixels = native(args.worker, query, source_control, load(fonts), '--pptx-resource-page')
                assert pixels == expected_pixels, name+' zero-scale versus independent empty slide'
                c['zeroAreaControlPixelSha256'] = sha(expected_pixels)
    for kind, cases in records.items():
        put(root/(kind+'.json'), dict(format='musteroffice.scale-playback-native/1', cases=cases))
    for p in inputs.values():
        load(p)
    put(root/'report.json', dict(status='passed', inputs=inputs, nativeFrames=sum(len(v) for v in records.values()), zeroAreaControls=3, scope='Native kernel and source-resource worker; Office/WPS and full PPT acceptance remain open.'))
    print(json.dumps(dict(status='passed', frames=sum(len(v) for v in records.values()))))

if __name__ == '__main__':
    main()
