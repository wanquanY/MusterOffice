"""Replay sealed programs in isolated output paths; compare exact historical bytes."""
import hashlib,json,subprocess
from pathlib import Path
repo=Path.cwd();root=Path('.codex-work/retained-timing');programs=root/'replay-programs';programs.mkdir(exist_ok=True)
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
seal=Path('docs/reviews/evidence/2026-09-26-container-lifecycle-verification.json');assert entry(seal)['sha256']=='6df136642857288790bd452214f15316305e33d4dddb42d3a850d06722f8ceb1'
old=json.loads(seal.read_text());sources={r['path']:r for r in old['sourceFiles']}
# Reuse the sealed full regression runner, redirecting only its current stage paths.
source=Path('tools/verification/container-lifecycle-replay.py');assert entry(source)==sources[str(source)]
program=programs/'prior.py';program.write_text(source.read_text().replace('.codex-work/container-lifecycle',str(root)))
with (root/'prior-replay.log').open('w')as log:subprocess.run(['python3',str(program)],stdout=log,stderr=subprocess.STDOUT,check=True)
records=json.loads((root/'regression.json').read_text())['cases']
# Add the immediately preceding container lifecycle corpus.
source=Path('tools/verification/container-lifecycle-parity.mjs');assert entry(source)==sources[str(source)]
output=root/'containers';output.mkdir(exist_ok=True)
for name in ['author-fixtures.json','source-fixtures.json']:(output/name).write_bytes(Path('.codex-work/container-lifecycle',name).read_bytes())
code=source.read_text().replace('.codex-work/container-lifecycle/wasm-node',str(root/'wasm-node')).replace("root='.codex-work/container-lifecycle'",f"root='{output}'").replace('../../.codex-work/',str(repo/'.codex-work')+'/').replace("entry(root+'/wasm-node/","entry('.codex-work/retained-timing/wasm-node/")
program=programs/'containers.mjs';program.write_text(code);log=root/'containers-replay.log'
with log.open('w')as out:subprocess.run(['node',str(program)],stdout=out,stderr=subprocess.STDOUT,check=True)
previous=Path('.codex-work/container-lifecycle/product.json');current=output/'product.json';prior=json.loads(previous.read_text());actual=json.loads(current.read_text());assert len(prior['cases'])==len(actual['cases'])
for a,b in zip(actual['cases'],prior['cases']):
 assert a['name']==b['name']
 for k in ['request','response','pixels','frame']:
  if k in b:assert Path(a[k]['path']).read_bytes()==Path(b[k]['path']).read_bytes(),(a['name'],k)
for a,b in zip(actual['exports'],prior['exports']):assert Path(a['output']['path']).read_bytes()==Path(b['output']['path']).read_bytes()
records.append(dict(name='containers',source=entry(source),program=entry(program),log=entry(log),previous=entry(previous),current=entry(current),pairedCalls=actual['pairedCalls']))
(root/'regression.json').write_text(json.dumps(dict(cases=records),indent=2)+'\n')
print(json.dumps(dict(cases=[dict(name=r['name'],pairedCalls=r['pairedCalls'])for r in records],frozenBytesIdentical=True)))
