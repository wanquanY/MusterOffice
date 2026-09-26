"""Audit all previous results, permitting only explicitly added visibility and visual-coverage metadata."""
import hashlib,json,zipfile
from pathlib import Path
ROOT=Path('.codex-work/source-page');OLD=Path('docs/reviews/evidence/2026-09-25-source-placement-verification.json')
read=lambda p:json.loads(Path(p).read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
old=read(OLD);reports={};same=changed=0;additions={'hidden':0,'showMasterShapes':0,'textBodyOrdinal':0,'visualIssues':0};archive=zipfile.ZipFile(ROOT/'previous-json.zip')
# Only the old-to-new additive raw projection is accepted; existing values,
# diagnostics, profiles, geometry, matrices and all raster digests stay exact.
def projection(a,b,where):
    if a==b:return
    if isinstance(a,list) and isinstance(b,list):
        assert len(a)==len(b),where
        for i,(x,y) in enumerate(zip(a,b,strict=True)):projection(x,y,where+'/'+str(i))
    elif isinstance(a,dict) and isinstance(b,dict):
        extra=b.keys()-a.keys();assert not a.keys()-b.keys(),where
        for k in extra:
            if k=='showMasterShapes':
                assert {'rootObjectId','objects','sha256','kind','links'}<=a.keys(),where
                assert isinstance(b[k],bool),where
            elif k in ['hidden','textBodyOrdinal']:
                assert {'nativeId','kind','paragraphs','resolution'}<=a.keys(),where
                assert isinstance(b[k],bool) if k=='hidden' else isinstance(b[k],int) and b[k]>=0,where
            elif k=='visualIssues':
                assert 'objects' in a or 'nativeId' in a,where
                for issue in b[k]:
                    assert set(issue)=={'sourceOrdinal','kind','namespace','localName'},where
                    assert isinstance(issue['sourceOrdinal'],int) and issue['sourceOrdinal']>=0,where
                    assert issue['kind'] in ['element','attribute'],where
            else:raise AssertionError((where,'unexpected field',k))
            additions[k]+=1
        for k in a:projection(a[k],b[k],where+'/'+k)
    else:raise AssertionError((where,a,b))
def case(a,b,where):
    if a==b:return False
    expected=dict(a)
    # Any changed file digest must have recoverable previous JSON bytes and a
    # currently verified path. Binary pixel/frame changes cannot enter here.
    for k in a:
        if k.endswith('Sha256') and a[k]!=b.get(k):
            field=k[:-6];path_key=field+'Path'
            assert path_key in a and path_key in b,(where,k)
            previous=archive.read(a[k]+'.json');assert sha(previous)==a[k]
            raw=Path(b[path_key]).read_bytes();assert sha(raw)==b[k]
            projection(json.loads(previous),json.loads(raw),where+'/'+field)
            expected[k]=b[k]
    projection(expected,b,where)
    return True
for name,entry in old['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert sha(snapshot.read_bytes())==entry['report']['sha256'];a=read(snapshot)
    path=ROOT/(name+'-regression.json') if name in {'document','opc','source','color','font','export'} else Path(entry['report']['path']);b=read(path)
    field='batches' if name=='bidiConformance' else 'cases';assert len(a[field])==len(b[field]),name
    n=0
    for i,(x,y) in enumerate(zip(a[field],b[field],strict=True)):
        if case(x,y,name+'/'+str(i)):n+=1
    # Extra input edit requests/receipts outside the primary case list remain
    # identical too; build identities are recorded separately in the new seal.
    for key in ['requests','faults','actualAllocationFailure','edits']:
        if key in a:projection(a[key],b[key],name+'/'+key)
    same+=len(a[field])-n;changed+=n
    reports[name]={'casesUnchanged':len(a[field])-n,'additiveVisualProjectionOnly':n,'reportPath':str(path)}
result={'format':'musteroffice.source-page-regression/1','previousEvidenceSha256':sha(OLD.read_bytes()),'unchangedBatches':same,'migratedBatches':changed,'additions':additions,'reports':reports}
(ROOT/'regression-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='reports'}))
