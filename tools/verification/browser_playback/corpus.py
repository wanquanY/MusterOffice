"""Prepare explicitly owned/pinned historical material for an isolated browser."""
import hashlib
import json
from pathlib import Path


def sha(data):
    return hashlib.sha256(data).hexdigest()


def prepare(author, source, native):
    inputs, assets, owners, expected = {}, {}, [], []

    def read(path):
        path = Path(path)
        data = path.read_bytes()
        inputs[str(path)] = sha(data)
        return data

    def load(record):
        data = read(record['path'])
        assert len(data) == record['byteLength'] and sha(data) == record['sha256']
        return data

    reference = json.loads(read(native / 'report.json'))
    assert reference['status'] == 'passed'
    for kind, manifest in [('author', author), ('source', source)]:
        groups = {}
        for case in json.loads(read(manifest))['cases']:
            old = json.loads(load(case['response']))
            if old['status'] != 'rendered':
                continue
            q = json.loads(load(case['request']))
            if kind == 'author':
                request = {k: q['playback'][k] for k in ['snapshot', 'slide', 'binding']}
                request.update(viewport=q['viewport'], defaults=q['defaults'])
                sample = dict(at=q['playback']['at'], history=q['playback'].get('history'))
            else:
                request = dict(page=q['page'], binding=q['sample']['binding'])
                sample = dict(at=q['sample']['at'], history=q['sample'].get('history'))
            groups.setdefault(json.dumps(request), []).append((case, sample))
        for group, (key, cases) in enumerate(groups.items()):
            owner = dict(kind=kind, request=json.loads(key), frames=[])
            previous = next(o for o in reference['observations'] if o['kind'] == kind and o['group'] == group)
            assert len(previous['frames']) == len(cases)
            if kind == 'source':
                for name in ['source', 'fonts']:
                    data = load(cases[0][0][name])
                    assert all(load(c[name]) == data for c, _ in cases)
                    url = f'assets/{sha(data)}.bin'
                    assets[url] = data
                    owner[name] = dict(url=url, byteLength=len(data), sha256=sha(data))
            for index, (case, sample) in enumerate(cases):
                directory = native / f'{kind}-{group:03}' / 'output'
                pixels = read(directory / f'{index:04}.rgba')
                info = json.loads(read(directory / f'{index:04}.json'))
                old_frame = previous['frames'][index]
                assert old_frame['name'] == case['name'] and old_frame['pixelSha256'] == sha(pixels)
                assert pixels == load(case['pixels'])
                frame = dict(index=len(expected), name=case['name'], sample=sample)
                owner['frames'].append(frame)
                expected.append(dict(**frame, pixels=pixels, info=info))
            owners.append(owner)
    assert len(expected) == reference['frameCount']
    return owners, expected, assets, inputs
