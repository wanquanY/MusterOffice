"""Replay the 100 frozen source-page Native requests without touching releases."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT=Path('.codex-work/source-text-page')
OLD=Path('docs/reviews/evidence/2026-09-25-text-body-verification.json')
CLI=Path('target/debug/mo-cli')
WORKER=Path('target/debug/mo-raster-worker')
def entry(path):
    p=Path(path);data=p.read_bytes()
    return dict(path=str(p),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())
assert entry(OLD)['sha256']=='0665a6ffd4bc4cbd31acc335ed6974963441f6a1e26850ead40d6b5c886c5158'
previous=json.loads(OLD.read_text())['regressionReports']['sourcePage']['report']
assert entry(previous['path'])==previous
report=json.loads(Path(previous['path']).read_text())
OUT=ROOT/'page-regressions';OUT.mkdir(exist_ok=True)
records=[];counts=dict(source=0,compile=0,raster=0,pixels=0)
with tempfile.TemporaryDirectory(prefix='page-cli-',dir=ROOT) as tmp:
    for i,case in enumerate(report['cases']):
        for key in ['source','request','response','pixels']:
            if key+'Path' in case:assert entry(case[key+'Path'])['sha256']==case[key+'Sha256']
        output=Path(tmp)/(str(i)+'.rgba')
        mode=case['kind']
        if mode=='source':args=['pptx-inspect',case['sourcePath']]
        elif mode=='compile':args=['compile-pptx-page',case['requestPath'],case['sourcePath']]
        else:
            assert mode=='raster'
            args=['render-pptx-page',case['requestPath'],case['sourcePath'],str(output)]
        done=subprocess.run([str(CLI),*args],capture_output=True,timeout=60)
        assert done.returncode==0,(case['name'],mode,done.stderr)
        assert done.stdout.rstrip(b'\n')==Path(case['responsePath']).read_bytes().rstrip(b'\n'),(case['name'],mode)
        current=OUT/(str(i)+'.json');current.write_bytes(done.stdout)
        item=dict(name=case['name'],mode=mode,source=entry(case['sourcePath']),priorResponse=entry(case['responsePath']),response=entry(current))
        if 'requestPath' in case:item['request']=entry(case['requestPath'])
        if 'pixelsPath' in case:
            assert output.read_bytes()==Path(case['pixelsPath']).read_bytes(),case['name']
            path=OUT/(str(i)+'.rgba');path.write_bytes(output.read_bytes())
            item.update(priorPixels=entry(case['pixelsPath']),pixels=entry(path));counts['pixels']+=1
        else:assert not output.exists()
        records.append(item);counts[mode]+=1
assert len(records)==100
result=dict(format='musteroffice.source-text-page-native-regressions/1',scope=__doc__,
            counts=counts,previousEvidence=entry(OLD),previousReport=previous,
            cli=entry(CLI),worker=entry(WORKER),cases=records)
(ROOT/'page-regressions.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(counts))
