"""Replay pinned, owned author/source pixels through an external SDK consumer.

Reference manifests are explicit inputs; their bytes are verified before use.
This is native SDK integration evidence, not complete PPT or Office acceptance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(record):
    data = Path(record['path']).read_bytes()
    assert len(data) == record['byteLength'] and sha(data) == record['sha256'], record['path']
    return data


def write(path, value):
    with path.open('x') as output:
        json.dump(value, output, indent=2); output.write('\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['output', 'consumer', 'worker', 'author', 'source']:
        parser.add_argument('--' + key, type=Path, required=True)
    args = parser.parse_args()
    root = args.output.resolve(); root.mkdir(parents=True, exist_ok=False)
    worker, consumer = args.worker.resolve(), args.consumer.resolve()
    worker_sha, consumer_sha = sha(worker.read_bytes()), sha(consumer.read_bytes())
    observations, inputs = [], {}
    for kind in ['author', 'source']:
        reference = getattr(args, kind)
        inputs[str(reference)] = sha(reference.read_bytes())
        records = json.loads(reference.read_text())['cases']
        groups = {}
        for case in records:
            old = json.loads(load(case['response']))
            if old['status'] != 'rendered':
                continue
            request = json.loads(load(case['request']))
            if kind == 'author':
                q = request['playback']
                prepare = {k: q[k] for k in ['snapshot', 'slide', 'binding']}
                prepare.update(viewport=request['viewport'], defaults=request['defaults'])
                sample = {k: q.get(k) for k in ['at', 'history']}
            else:
                prepare = dict(page=request['page'], binding=request['sample']['binding'])
                sample = {k: request['sample'].get(k) for k in ['at', 'history']}
            key = json.dumps(prepare, sort_keys=True)
            groups.setdefault(key, []).append((case, sample, old))
        for ordinal, (key, cases) in enumerate(groups.items()):
            directory = root / f'{kind}-{ordinal:03}'; directory.mkdir()
            prepare = directory / 'prepare.json'; write(prepare, json.loads(key))
            samples = directory / 'samples.json'; write(samples, [c[1] for c in cases])
            output = directory / 'output'
            command = [str(consumer), str(worker), worker_sha, kind, str(prepare), str(samples), str(output)]
            if kind == 'source':
                for name in ['source', 'fonts']:
                    data = load(cases[0][0][name])
                    path = directory / (name + '.bin'); path.write_bytes(data)
                    assert all(load(c[0][name]) == data for c in cases)
                    command.append(str(path))
            result = subprocess.run(command, cwd=directory, env={'PATH': str(directory/'empty-path')},
                                    capture_output=True, timeout=60)
            (directory/'stdout.log').write_bytes(result.stdout); (directory/'stderr.log').write_bytes(result.stderr)
            assert result.returncode == 0 and not result.stderr, result.stderr.decode(errors='replace')
            summary = json.loads((output/'result.json').read_text())
            assert summary['status'] == 'computed' and summary['productCommitted'] is False
            session = summary['session']
            assert session['prepared']['planId'] == session['advanced']['planId']
            assert int(session['advanced']['binding']['generation']) == int(session['prepared']['binding']['generation']) + 1
            frames = []
            for (case, sample, old), files in zip(cases, summary['files'], strict=True):
                info = json.loads((output/files['metadata']).read_text())
                pixels = (output/files['pixels']).read_bytes()
                assert pixels == load(case['pixels']), case['name']
                if kind == 'author':
                    assert info == old['info'], case['name']
                else:
                    assert info['playback'] == old['info']['playback'], case['name']
                    assert info['page']['page'] == old['info']['page']['page'], case['name']
                    assert info['page']['textWork']['componentCalls'] == 0
                    assert info['page']['textWork']['fontUploadBytes'] == 0
                    assert info['page']['gatherCopyBytes'] == 0
                for ref in ['request', 'response', 'pixels', *( ['source', 'fonts'] if kind == 'source' else [])]:
                    inputs[case[ref]['path']] = case[ref]['sha256']
                frames.append(dict(name=case['name'],pixelSha256=sha(pixels),byteLength=len(pixels)))
            observations.append(dict(kind=kind, group=ordinal, frames=frames,
                                     planId=session['prepared']['planId'], disposed=True))
    assert sha(worker.read_bytes()) == worker_sha and sha(consumer.read_bytes()) == consumer_sha
    assert all(sha(Path(path).read_bytes()) == digest for path, digest in inputs.items())
    report = dict(format='musteroffice.sdk-playback-reference/1',status='passed',
                  consumerSha256=consumer_sha,workerSha256=worker_sha,observations=observations,
                  frameCount=sum(len(o['frames']) for o in observations),inputs=inputs,
                  scope='Frozen owned fixture bytes through copied SDK and native worker. No product, Office/WPS or complete playback claim.')
    write(root/'report.json',report)
    print(json.dumps(dict(status=report['status'],owners=len(observations),frames=report['frameCount'])))


if __name__ == '__main__':
    main()
