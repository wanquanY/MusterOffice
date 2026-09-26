"""Prove v2 changes only typed provenance/profile for every previous geometry result."""
import hashlib,json
from pathlib import Path
ROOT=Path('.codex-work/preset-expansion')
OLD=Path('docs/reviews/evidence/2026-09-25-gradient-raster-verification.json')
read=lambda p:json.loads(Path(p).read_text())
sha=lambda b:hashlib.sha256(b).hexdigest()
old=read(OLD);reports={};unchanged=migrated=0
BASE={'document','opc','source','color','font','export'}
MIGRATED={'geometryEvaluation','geometryApplicationProbes','nativePaths','nativePathGuides'}
def normalize(v):
    if isinstance(v,str):return v.replace('ecma376-2016-ms-presets-draft-v2','ecma376-2016-ms-guides-draft-v1')
    if isinstance(v,list):return [normalize(x) for x in v]
    if not isinstance(v,dict):return v
    out={}
    for k,x in v.items():
        if k=='origin':
            assert x.keys()=={'kind','sourceOrdinal'} and x['kind']=='document',x
            out['sourceOrdinal']=x['sourceOrdinal']
        elif k in ['guideX','guideY','guideRadius','guideAngle'] and isinstance(x,dict):
            assert x.keys()=={'kind','sourceOrdinal'} and x['kind']=='document',x
            out[k]=x['sourceOrdinal']
        else:out[k]=normalize(x)
    return out
for name,entry in old['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert sha(snapshot.read_bytes())==entry['report']['sha256'];before=read(snapshot)
    path=ROOT/(name+'-regression.json') if name in BASE else Path(entry['report']['path']);after=read(path)
    field='batches' if name=='bidiConformance' else 'cases'
    if name not in MIGRATED:
        assert before[field]==after[field],name;unchanged+=len(after[field]);reports[name]={'casesUnchanged':len(after[field]),'reportPath':str(path)};continue
    assert len(before[field])==len(after[field]),name
    for a,b in zip(before[field],after[field]):
        expected=dict(a)
        for key in ['request','response','evaluation','evaluationRequest']:
            if key+'Path' not in a:continue
            raw=Path(b[key+'Path']).read_bytes();assert sha(raw)==b[key+'Sha256']
            previous=ROOT/'previous-responses'/(a[key+'Sha256']+'.json');assert sha(previous.read_bytes())==a[key+'Sha256']
            assert normalize(json.loads(raw))==json.loads(previous.read_bytes()),(name,a['name'],key)
            expected[key+'Sha256']=b[key+'Sha256']
        assert expected==normalize(b),(name,a['name']);migrated+=1
    reports[name]={'typedProvenanceAndProfileOnly':len(after[field]),'reportPath':str(path)}
result={'format':'musteroffice.preset-expansion-regression/1','previousEvidenceSha256':sha(OLD.read_bytes()),'unchangedBatches':unchanged,'migratedBatches':migrated,'reports':reports}
(ROOT/'regression-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'unchangedBatches':unchanged,'migratedBatches':migrated,'reports':len(reports)}))
