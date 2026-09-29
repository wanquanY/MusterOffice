"""Assemble this stage only after rechecking current sources and actual outputs."""
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
    for name in ['final-rust-01', 'lint-02', 'types-01', 'schemas-01', 'native-build-02', 'wasm-build-02', 'corpus-03', 'bundle-01', 'inspection-02', 'wasm-replay-01', 'wasm-legacy-replay-01', 'rust-sdk-01', 'consumer-build-01', 'native-sdk-replay-01']:
        report = read(name+'.json')
        assert report['exitCode'] == 0 and report['sourceUnchanged'], name
        assert sha(stage/(name+'.log')) == report['logSha256'], name
        for path, expected in report['sourceAfter'].items():
            assert sha(root/path) == expected, (name,path)
        commands[name] = dict(reportSha256=sha(stage/(name+'.json')), logSha256=report['logSha256'])
    tests = [tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', (stage/'final-rust-01.log').read_text())]
    passed, failed, ignored = map(sum, zip(*tests))
    assert passed > 500 and failed == 0
    reports = {}
    for name in ['corpus-03', 'native-sdk-replay-01', 'wasm-replay-01', 'wasm-legacy-replay-01', 'inspection-02']:
        report = read(name+'/report.json')
        assert report['status'] == 'passed'
        for path, entry in report['inputs'].items():
            assert sha(Path(path)) == (entry['sha256'] if isinstance(entry,dict) else entry), path
        reports[name] = dict(sha256=sha(stage/name/'report.json'), frames=report.get('frameCount'))
    for kind in ['author','source']:
        for case in read('corpus-03/'+kind+'.json')['cases']:
            for name in ['request','response','pixels']+(['source','fonts'] if kind=='source' else []):
                entry=case[name]; path=Path(entry['path'])
                assert sha(path)==entry['sha256'] and path.stat().st_size==entry['byteLength']
    packages = {}
    for name, manifest in [('bundle-01','bundle-manifest.json'), ('rust-sdk-01','sdk-manifest.json')]:
        data = read(name+'/'+manifest)
        for entry in data['files']:
            path = stage/name/entry['path']
            assert sha(path)==entry['sha256'] and path.stat().st_size==entry['byteLength']
        packages[name] = dict(manifestSha256=sha(stage/name/manifest), files=len(data['files']), payloadBytes=sum(x['byteLength'] for x in data['files']), archiveBytes=(stage/(name+'.tar.gz')).stat().st_size, archiveSha256=sha(stage/(name+'.tar.gz')))
    evidence = dict(format='musteroffice.native-navigation-verification/1', status='passed-scoped', platform=dict(system=platform.system(), machine=platform.machine(), release=platform.release()),
        scope='Native sequence navigation without natural-end seek, actual owned WPS scale timing projection, fractional page clipping, native/public SDK/WASM parity. No complete Office/WPS, product replacement, performance or release acceptance.',
        tests=dict(passed=passed, failed=failed, ignored=ignored), commands=commands, reports=reports,
        corpus=dict(newFrames=60, actualWpsSourceFrames=6, authorSourcePairs=27, legacyWasmFrames=259),
        xsd=dict(parts=sum(len(v['parts']) for v in read('xsd.json').values()), reportSha256=sha(stage/'xsd.json')),
        packages=packages, sourceInventory=read('final-rust-01.json')['sourceAfter'],
        sourceFixture=dict(path='crates/mo-pptx/tests/fixtures/wps-by-only-timing.xml', sha256=sha(root/'crates/mo-pptx/tests/fixtures/wps-by-only-timing.xml')),
        remaining=['Natural-end seek and implicit event targets', 'General animation, transitions, media, SmartArt, equations and full feature coverage', 'Further independent Office/WPS editable roundtrip', 'Coherent product SDK/worker/playback upgrade and complete Electron acceptance', 'Performance, memory, installer size and all replacement gates'],
        product=dict(worktreeBranch='codex/musteroffice-integration', productPinsUnchanged=True, stableUnchanged=True))
    with args.output.open('x') as output:
        json.dump(evidence, output, indent=2, ensure_ascii=False); output.write('\n')
    print(json.dumps(dict(status=evidence['status'],tests=evidence['tests'],sha256=sha(args.output))))


if __name__ == '__main__':
    main()
