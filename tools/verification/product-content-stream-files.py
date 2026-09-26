"""Inspect actual product-storage readback with the unchanged shared kernel."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
args = parser.parse_args()
stage = Path('.codex-work/product-content-stream')
actual = stage / 'actual'
fixture = args.product_repo / 'packages/agent-runtime-product-client/tests/fixtures/office-native'


def read(path):
    return json.loads(Path(path).read_text())


def entry(path):
    data = Path(path).read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


prior = read('docs/reviews/evidence/2026-09-26-product-office-manifest-verification.json')
records = {p['path']: p for p in prior['productPrivateFiles']}
for name in ['files.json', 'manifest.json', 'bundle.json']:
    path = fixture / name
    expected = records[path.relative_to(args.product_repo).as_posix()]
    assert entry(path)['sha256'] == expected['sha256']
index = read(actual / 'files.json')
assert index == read(fixture / 'files.json') and len(index) == 13
for item in index:
    data = (actual / item['file']).read_bytes()
    assert data == (fixture / item['file']).read_bytes()
    assert hashlib.sha256(data).hexdigest() == item['content']['sha256']
    assert len(data) == int(item['content']['byte_length'])
manifest = read(fixture / 'manifest.json')
bundle = read(actual / 'bundle.json')
pins = manifest['kernel']
expected = dict(documentId=pins['document_id'], revision=pins['revision'],
                semanticDigest=pins['semantic_digest'], settingsDigest=pins['settings_sha256'],
                renderer=dict(profile=pins['renderer_profile'], implementationSha256=pins['renderer_sha256']))
filenames = {p['content']['content_id']: p['file'] for p in index}
contents = []
with (stage / 'stored-contents.bin').open('xb') as stream:
    offset = 0
    for asset in manifest['assets']:
        with (actual / filenames[asset['content']['content_id']]).open('rb') as source:
            while data := source.read(65536):
                stream.write(data)
        size = int(asset['content']['byte_length'])
        contents.append(dict(assetId=asset['id'], byteOffset=str(offset), byteLength=str(size)))
        offset += size
request = dict(bundle=bundle, expected=expected, contents=contents)
(stage / 'inspect.json').write_text(json.dumps(request, sort_keys=True) + '\n')
cli = Path('.codex-work/embedded-export/frozen/mo-cli')
export_evidence = read('docs/reviews/evidence/2026-09-26-embedded-export-verification.json')
# The binary is an explicitly frozen input in the earlier stage, not a newly
# compiled current CLI or a test-only reimplementation of delivery validation.
assert any(p.get('path') == str(cli) and p == entry(cli)
           for p in export_evidence['artifacts'])


def inspect(name, request_path, content_path):
    completed = subprocess.run([str(cli), 'delivery-inspect', str(request_path), str(content_path)],
                               capture_output=True, check=True)
    assert not completed.stderr
    (stage / f'{name}.json').write_bytes(completed.stdout)
    return json.loads(completed.stdout)


response = inspect('inspected', stage / 'inspect.json', stage / 'stored-contents.bin')
assert response['status'] == 'inspected'
assert response['report'] == read('.codex-work/embedded-export/actual/inspection.json')
damaged = bytearray((stage / 'stored-contents.bin').read_bytes())
damaged[-1] ^= 1
(stage / 'corrupted-contents.bin').write_bytes(damaged)
negative = inspect('rejected-corrupt', stage / 'inspect.json', stage / 'corrupted-contents.bin')
assert negative['status'] == 'error'
request['expected']['renderer']['implementationSha256'] = '0' * 64
(stage / 'wrong-renderer.json').write_text(json.dumps(request, sort_keys=True) + '\n')
wrong = inspect('rejected-renderer', stage / 'wrong-renderer.json', stage / 'stored-contents.bin')
assert wrong['status'] == 'error'
report = dict(actualFiles=13, assets=12, pages=response['report']['pages'],
              allBytesUnchanged=True, finalStoredInspectionUnchanged=True,
              corruptFinalBytesRejected=True, wrongExternalRendererRejected=True,
              cli=entry(cli), files=[entry(actual / p['file']) for p in index])
(stage / 'file-verification.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(dict(files=13, assets=12, pages=report['pages'], negatives=2)))
