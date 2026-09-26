"""Rerun the frozen playback suite in a fresh directory, then compare every result."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path(sys.argv[1])
assert root.parent == Path('.codex-work') and not root.exists()
root.mkdir()

def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

def load(record):
    assert entry(record['path']) == record
    return Path(record['path']).read_bytes()

parent_path = Path('docs/reviews/evidence/2026-09-26-delivery-receive-verification.json')
assert entry(parent_path)['sha256'] == 'd5b8fefe0d75bbd7efc8749cf38d8c584bfd1993f99e554df87963695d89e446'
parent = json.loads(parent_path.read_text())
script = 'tools/verification/retained-timing-parity.mjs'
original = load(next(r for r in parent['sourceFiles'] if r['path'] == script)).decode()
assert original.count("const root='.codex-work/retained-timing'") == 1
assert original.count('../../.codex-work/retained-timing/wasm-node/mo_wasm.js') == 1
assert original.count('target/debug/mo-cli') == 2
assert original.count('target/debug/mo-raster-worker') == 3
assert original.count("root+'/wasm-node/mo_wasm_bg.wasm'") == 1
source = original.replace("const root='.codex-work/retained-timing'", f"const root='{root}'")
source = source.replace('../../.codex-work/retained-timing/wasm-node/mo_wasm.js', '../../.codex-work/delivery-size/wasm-package-read/mo_wasm.js')
source = source.replace('target/debug/mo-cli', '.codex-work/delivery-size/frozen/mo-cli')
source = source.replace('target/debug/mo-raster-worker', '.codex-work/delivery-size/frozen/mo-raster-worker')
source = source.replace("root+'/wasm-node/mo_wasm_bg.wasm'", "'.codex-work/delivery-size/wasm-package-read/mo_wasm_bg.wasm'")
driver = root/'driver.mjs'
driver.write_text(source)
with (root/'run.log').open('x') as log:
    result = subprocess.run(['node', str(driver)], stdout=log, stderr=subprocess.STDOUT)
assert result.returncode == 0, (root/'run.log').read_text()[-4000:]

old_path = Path('.codex-work/retained-timing/product.json')
old_evidence_path = Path('docs/reviews/evidence/2026-09-26-retained-timing-verification.json')
assert entry(old_evidence_path)['sha256'] == 'e9408007e7d4d7a28bf44f454acbb969e0d9371f8ba7cc896f9c4a91e5ee1ff6'
old_evidence = json.loads(old_evidence_path.read_text())
before = json.loads(load(old_evidence['reports']['product']))
after_path = root/'product.json'
after = json.loads(after_path.read_text())
assert len(before['cases']) == len(after['cases']) == 654
for old, new in zip(before['cases'], after['cases'], strict=True):
    for key in ['name', 'operation', 'owner', 'rasters', 'decodes', 'shapes']:
        assert old.get(key) == new.get(key), (key, new['name'])
    for key in ['request', 'response', 'pixels', 'frame', 'source', 'fonts']:
        assert (key in old) == (key in new), (key, new['name'])
        if key in old:
            assert load(old[key]) == load(new[key]), (key, new['name'])
report = dict(format='musteroffice.package-read-playback/1', parent=entry(parent_path),
    driver=entry(driver), originalDriver=entry(script), beforeEvidence=entry(old_evidence_path), before=entry(old_path), after=entry(after_path),
    pairedCalls=654, allPreviousRequestsResponsesPixelsFramesAndComponentCallsIdentical=True,
    scope='Actual Native/WASM authored and imported playback, retained preparation, invalidation and disposal; unchanged frozen results, not complete animation or real-time player acceptance.')
(root/'comparison.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(dict(pairedCalls=654, unchanged=True, report=str(root/'comparison.json'))))
