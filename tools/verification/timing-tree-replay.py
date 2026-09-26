"""Run sealed regression programs against current artifacts without rewriting history."""
import hashlib,json,subprocess
from pathlib import Path
repo=Path.cwd();root=Path('.codex-work/timing-tree');dest=root/'regression';dest.mkdir(exist_ok=True);programs=root/'replay-programs';programs.mkdir(exist_ok=True)
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
seal=Path('docs/reviews/evidence/2026-09-26-source-session-verification.json');assert entry(seal)['sha256']=='5caa25432321b1b8f8904c74e534205fd8c2b5cbc66730c998ad3978ac804283'
old=json.loads(seal.read_text());sources={c['path']:c for c in old['sourceFiles']}
(dest/'fixtures.json').write_bytes(Path('.codex-work/source-session/fixtures.json').read_bytes())
records=[]
for name,report in [('replay','replay.json'),('author-replay','session/product.json'),('source-replay','oneshot/product.json'),('parity','product.json')]:
 source=Path(f'tools/verification/source-session-{name}.mjs');assert entry(source)==sources[str(source)]
 code=source.read_text().replace('.codex-work/source-session/wasm-node','.codex-work/timing-tree/wasm-node').replace('.codex-work/source-session',str(dest)).replace('../../.codex-work/',str(repo/'.codex-work')+'/').replace("root+'/wasm-node/", "'.codex-work/timing-tree/wasm-node/")
 program=programs/(name+'.mjs');program.write_text(code)
 log=root/(name+'-regression.log')
 with log.open('w')as out:r=subprocess.run(['node',str(program)],stdout=out,stderr=subprocess.STDOUT)
 if r.returncode:raise SystemExit(f'{name} exited {r.returncode}: {log}')
 current=json.loads((dest/report).read_text());prior=json.loads((Path('.codex-work/source-session')/report).read_text());assert len(current['cases'])==len(prior['cases'])
 for actual,expected in zip(current['cases'],prior['cases']):
  assert actual['name']==expected['name']
  for key in ['request','response','pixels','frame']:
   if key in expected:
    assert key in actual
    assert Path(actual[key]['path']).read_bytes()==Path(expected[key]['path']).read_bytes(),(name,actual['name'],key)
 records.append(dict(name=name,source=entry(source),program=entry(program),log=entry(log),previous=entry(Path('.codex-work/source-session')/report),current=entry(dest/report),pairedCalls=current['pairedCalls']))
 print(json.dumps(dict(name=name,pairedCalls=current['pairedCalls'],frozenBytesIdentical=True)),flush=True)
(root/'regression.json').write_text(json.dumps(dict(cases=records),indent=2)+'\n')
