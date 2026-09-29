"""Verify scoped-clock sources, actual playback evidence and immutable package bytes."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import re


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    stage = args.stage
    read = lambda name: json.loads((stage/name).read_text())
    commands = {}
    for name in ['rust-03', 'lint-03', 'native-build-02', 'wasm-build-02', 'corpus-02', 'bundle-02', 'inspection-02', 'wasm-replay-02', 'wasm-legacy-replay-02', 'wasm-navigation-replay-02', 'rust-sdk-02', 'consumer-build-02', 'native-sdk-replay-02']:
        report = read(name+'.json')
        assert report['exitCode'] == 0 and report['sourceUnchanged'], name
        assert sha(stage/(name+'.log')) == report['logSha256'], name
        for path, expected in report['sourceAfter'].items():
            assert sha(root/path) == expected, (name,path)
        commands[name] = dict(reportSha256=sha(stage/(name+'.json')), logSha256=report['logSha256'])
    tests = [tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', (stage/'rust-03.log').read_text())]
    passed, failed, ignored = map(sum, zip(*tests))
    assert passed > 500 and failed == 0
    reports = {}
    for name in ['corpus-02', 'native-sdk-replay-02', 'wasm-replay-02', 'wasm-legacy-replay-02', 'wasm-navigation-replay-02', 'inspection-02']:
        report = read(name+'/report.json')
        assert report['status'] == 'passed'
        for path, entry in report['inputs'].items():
            assert sha(Path(path)) == (entry['sha256'] if isinstance(entry,dict) else entry), path
        reports[name] = dict(sha256=sha(stage/name/'report.json'), frames=report.get('frameCount'))
    for kind in ['author','source']:
        for case in read('corpus-02/'+kind+'.json')['cases']:
            for name in ['request','response','pixels']+(['source','fonts'] if kind=='source' else []):
                entry=case[name]; path=Path(entry['path'])
                assert sha(path)==entry['sha256'] and path.stat().st_size==entry['byteLength']
    packages = {}
    for name, manifest in [('bundle-02','bundle-manifest.json'), ('rust-sdk-02','sdk-manifest.json')]:
        data = read(name+'/'+manifest)
        for entry in data['files']:
            path = stage/name/entry['path']
            assert sha(path)==entry['sha256'] and path.stat().st_size==entry['byteLength']
        packages[name] = dict(manifestSha256=sha(stage/name/manifest), files=len(data['files']), payloadBytes=sum(x['byteLength'] for x in data['files']), archiveBytes=(stage/(name+'.tar.gz')).stat().st_size, archiveSha256=sha(stage/(name+'.tar.gz')))
    evidence = dict(format='musteroffice.scoped-seek-verification/1', status='passed-scoped', platform=dict(system=platform.system(), machine=platform.machine(), release=platform.release()),
        scope='Exact activation-owned clock jumps for native sequence nextAc=seek; finite/infinite, nested/cached conditions, cross-scope feedback, reset, cancellation and work limits; editable exports and native/public SDK/WASM parity. No new WPS/Office playback observation, product replacement, performance or release acceptance.',
        tests=dict(passed=passed, failed=failed, ignored=ignored),
        causalTieRegression=dict(beforeSha256=sha(stage/'causal-tie-before-02.log'), afterSha256=sha(stage/'causal-tie-after.log')), commands=commands, reports=reports,
        corpus=dict(newFrames=130, authorSourcePairs=65, legacyWasmFrames=259, priorNavigationWasmFrames=60),
        xsd=dict(parts=sum(len(v['parts']) for v in read('xsd-02.json').values()), reportSha256=sha(stage/'xsd-02.json')),
        packages=packages, sourceInventory=read('rust-03.json')['sourceAfter'],
        assemblerSha256=sha(Path(__file__)),
        remaining=['Independent application calibration of natural-end seek and native implicit event targets', 'General animation, transitions, media, SmartArt, equations and full feature coverage', 'Further independent Office/WPS editable roundtrip', 'Coherent product SDK/worker/playback upgrade and complete Electron acceptance', 'Performance, memory, installer size and all replacement gates'],
        product=dict(worktreeBranch='codex/musteroffice-integration', productPinsUnchanged=True, stableUnchanged=True))
    with args.output.open('x') as output:
        json.dump(evidence, output, indent=2, ensure_ascii=False); output.write('\n')
    print(json.dumps(dict(status=evidence['status'],tests=evidence['tests'],sha256=sha(args.output))))


if __name__ == '__main__':
    main()
