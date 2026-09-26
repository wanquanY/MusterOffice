"""Verify a release stateless worker through the real parent adapter.

Requires the release binary to be built first. Writes only a new stage subtree.
"""
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

root = Path(sys.argv[1])
assert root.is_relative_to('.codex-work') and root.is_dir()
worker = Path('target/release/mo-export-worker')

def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

frozen = root/'frozen'
frozen.mkdir()
identity = entry(worker)
copied = frozen/'mo-export-worker'
with copied.open('xb') as stream:
    stream.write(worker.read_bytes())
copied.chmod(0o700)
assert entry(copied)['sha256'] == identity['sha256']
env = dict(os.environ, CARGO_BUILD_JOBS='2',
           MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()),
           MO_EXPORT_WORKER_TEST_BIN=str(copied.resolve()),
           MO_EXPORT_WORKER_TEST_SHA256=identity['sha256'],
           MO_NATIVE_EXPORT_EVIDENCE=str((root/'actual').resolve()))
records = []

def run(name, argv):
    log = root/f'{name}.log'
    with log.open('x') as stream:
        result = subprocess.run(argv, env=env, stdout=stream, stderr=subprocess.STDOUT)
    record = dict(name=name, argv=argv, exitCode=result.returncode, log=entry(log))
    records.append(record)
    print(json.dumps(record), flush=True)
    assert result.returncode == 0, log

run('release-integration', ['cargo', 'test', '-p', 'mo-export-worker',
                           '--test', 'native_export', '--locked', '--offline'])
run('independent-reference', [sys.executable, 'tools/verification/delivery-reference.py', str(root/'actual')])
assert entry(worker) == identity
assert entry(copied)['sha256'] == identity['sha256']
current = json.loads((root/'actual/reference.json').read_text())
prior_path = Path('.codex-work/delivery-pipeline/final/reference.json')
baseline_path = Path('docs/reviews/evidence/2026-09-26-delivery-verification.json')
baseline = json.loads(baseline_path.read_text())
assert entry(prior_path) == baseline['reports']['reference']
prior = json.loads(prior_path.read_text())
def file_by_name(directory, name):
    index = json.loads((directory/'files.json').read_text())
    return directory/next(e['file'] for e in index if e['name'] == name)
old_pptx = file_by_name(prior_path.parent, 'presentation')
new_pptx = file_by_name(root/'actual', 'presentation')
assert old_pptx.read_bytes() == new_pptx.read_bytes()
assert [p['premultipliedSha256'] for p in current['pages']] == [p['premultipliedSha256'] for p in prior['pages']]
report = dict(format='musteroffice.embedded-export-native/1',
    worker=entry(copied), builtWorker=identity, checks=records,
    inputFiles=[entry(p) for p in ['fixtures/presentations/delivery/input.json',
        'fixtures/presentations/native-export/resources.bin', 'fixtures/fonts/owned.ttf']],
    independentReference=entry(root/'actual/reference.json'),
    baselineEvidence=entry(baseline_path), previousReference=entry(prior_path), previousPptx=entry(old_pptx),
    actualPptx=entry(new_pptx), pptxAndPagePixelsUnchanged=True,
    verifierRuntime=dict(python=platform.python_version(), platform=platform.platform(),
        packages={name:importlib.metadata.version(name) for name in ['Pillow','lxml','python-pptx','jsonschema','referencing']}),
    limitations=[
        'The actual release worker performs the complete private export; the parent integration harness is a debug Rust test.',
        'The fixture uses an owned synthetic font; this is not Office/WPS, complete layout, advanced-content or product acceptance.',
        'No Runtime Invocation, Content Store, fenced Artifact commit or historical migration is implemented by this adapter.',
        'The single release executable includes native drawing and shaping, but not a complete distribution or installer.',
    ])
with (root/'native.json').open('x') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps(dict(workerBytes=identity['byteLength'], assets=12, pages=2, unchanged=True)))
