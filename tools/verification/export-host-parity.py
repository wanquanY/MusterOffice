"""Replay frozen public-call verification against current binaries and exports."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path('.codex-work/export-host')
parent_path = Path('docs/reviews/evidence/2026-09-26-delivery-verification.json')
parent = json.loads(parent_path.read_text())


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(parent_path)['sha256'] == '0526033a5f818a1bb0a91a1915b204fa9f5a94a91dbb386ac36158de5a94b6cb'
script = Path('tools/verification/delivery-parity.mjs')
assert entry(script) == next(r for r in parent['sourceFiles'] if r['path'] == str(script))
old = "const root='.codex-work/delivery-pipeline';"
source = script.read_text()
assert source.count(old) == 1
replay = root/'parity-replay.mjs'
with replay.open('x') as f:
    f.write(source.replace(old, "const root='.codex-work/export-host';"))
with (root/'parity-run.log').open('x') as f:
    run = subprocess.run(['node',str(replay),sys.argv[1]],stdout=f,stderr=subprocess.STDOUT)
assert run.returncode == 0
parity = json.loads((root/'parity.json').read_text())
previous_record = parent['reports']['parity']
assert entry(previous_record['path']) == previous_record
previous = json.loads(Path(previous_record['path']).read_text())
counts = {}
for name in ['authored', 'source', 'editor']:
    a_record = previous['reports'][name]
    assert entry(a_record['path']) == a_record
    a = json.loads(Path(a_record['path']).read_text())
    b = json.loads((root/f'{name}-parity.json').read_text())
    # Ignore only diagnostic output paths; compare every semantic response and
    # exported byte digest, including failures. Do not reduce this to counts.
    def normalize(cases):
        # Only the top-level fixture output locations may vary. Preserve every
        # field inside actual responses, including any named request/output.
        return [{k:v for k,v in case.items() if k not in ['request', 'output']}
                for case in cases]
    assert normalize(a['cases']) == normalize(b['cases']), name
    if name == 'source':
        assert normalize(a['outputs']) == normalize(b['outputs'])
    counts[name] = b['passed']
for a,b in zip(previous['pages'],parity['pages'],strict=True):
    for key in ['request','response','pixels']:
        assert entry(a[key]['path']) == a[key]
        assert entry(b[key]['path']) == b[key]
        assert a[key]['sha256'] == b[key]['sha256']
report = dict(format='musteroffice.export-host-parity/1', previous=entry(parent_path),
    frozenProgram=entry(script), replayProgram=entry(replay), current=entry(root/'parity.json'),
    calls=counts, pagePairs=len(parity['pages']), allPreviousCasesUnchanged=True,
    limitation='Existing file/edit and individual page APIs; full WASM export owner execution is not covered.')
with (root/'parity-reference.json').open('x') as f:
    json.dump(report,f,indent=2); f.write('\n')
print(json.dumps(dict(calls=counts,pagePairs=len(parity['pages']))))
