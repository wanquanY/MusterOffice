"""Check the shared native staging path with a frozen actual raster output."""
import hashlib
import json
from pathlib import Path
import subprocess

root=Path('.codex-work/sealed-export/publication');root.mkdir()
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
parent=json.loads(Path('docs/reviews/evidence/2026-09-26-retained-timing-verification.json').read_text())
worker=parent['currentArtifacts']['nativeWorker'];assert entry(worker['path'])==worker
replay=json.loads(Path('.codex-work/gradient-coordinates/replay.json').read_text())
case=next(c for c in replay['cases'] if c['name']=='new-center-point')
for k in ['request','response','pixels']:assert entry(case[k]['path'])==case[k]
dest=root/'rendered.rgba'
args=['target/debug/mo-cli','render-paths',case['request']['path'],str(dest)]
run=subprocess.run(args,capture_output=True,check=True)
assert run.stderr==b'' and run.stdout.rstrip(b'\n')==Path(case['response']['path']).read_bytes().rstrip(b'\n')
assert dest.read_bytes()==Path(case['pixels']['path']).read_bytes()
blocked=subprocess.run(args,capture_output=True)
assert blocked.returncode!=0 and blocked.stdout==b''
assert dest.read_bytes()==Path(case['pixels']['path']).read_bytes()
assert list(root.iterdir())==[dest]
report=dict(format='musteroffice.sealed-export-publication/1',scope='Shared CLI spool verified with existing raster worker; no new renderer or frame coverage.',cli=entry('target/debug/mo-cli'),worker=worker,request=case['request'],referenceResponse=case['response'],referencePixels=case['pixels'],output=entry(dest),actualCalls=2,pixelsAndMetadataUnchanged=True,existingDestinationUnchanged=True,noStagingLeftovers=True)
Path('.codex-work/sealed-export/publication.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:report[k] for k in ['actualCalls','pixelsAndMetadataUnchanged','existingDestinationUnchanged','noStagingLeftovers']}))
