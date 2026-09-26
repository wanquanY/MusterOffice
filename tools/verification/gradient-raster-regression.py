"""Exact migration audit of old solid-paint requests, frames, plans and pixels.

Only the declared Brush/ABI/profile/zero-work additions may change. Everything
else is compared to the actual previous request/response/frame snapshots.
"""
import hashlib,json,struct
from pathlib import Path
ROOT=Path('.codex-work/gradient-raster')
BASE=Path('docs/reviews/evidence/2026-09-25-fill-color-verification.json')
read=lambda p:json.loads(Path(p).read_text())
sha=lambda b:hashlib.sha256(b).hexdigest()
old_profile='skia-8d6d37b-q32-local-paths-srgb-premul-rgba8-v3-draft'
new_profile='skia-8d6d37b-q32-world-brushes-srgb-premul-rgba8-v4-draft'
zero_work={'gradients':0,'gradientStops':0,'gradientDraws':0,'gradientCoordinateErrorBound':'0','gradientValueErrorBound':0.0}
def old_frame(raw):
    words=list(struct.unpack('<'+'I'*(len(raw)//4),raw));assert words[1]==4 and words[9]==0
    pos=10
    for _ in range(words[5]):pos+=2+7*words[pos+1]
    pos+=4*words[8];assert pos+6*words[6]==len(words)
    result=words[:pos];result[1]=3
    for start in range(pos,len(words),6):assert words[start+5]==0;result+=words[start:start+5]
    return struct.pack('<'+'I'*len(result),*result)
def restore(value,frames):
    if isinstance(value,list):return [restore(v,frames) for v in value]
    if not isinstance(value,dict):return value
    out={}
    if 'gradients' in value:
        assert all(value[k]==v for k,v in zero_work.items()),value
        assert 'drawnCommands' in value and 'miterLimitErrorBound' in value
    for k,v in value.items():
        if k in zero_work and 'gradients' in value:continue
        if k=='profile' and v==new_profile:v=old_profile
        if k=='frameSha256':
            assert v in frames,v;v=frames[v]
        if k=='brush':
            assert 'path' in value and ('origin' in value or 'transform' in value)
            assert set(v)=={'kind','rgba'} and v['kind']=='solid'
            k,v='color',v['rgba']
        out[k]=restore(v,frames)
    return out
counts={'unchangedBatches':0,'migratedBatches':0,'identicalSolidPixels':0,'exactOldFramesReconstructed':0,'plansChecked':0}
records={}
for name,record in read(BASE)['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert sha(snapshot.read_bytes())==record['report']['sha256']
    path=ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(record['report']['path'])
    before,after=read(snapshot),read(path);field='batches' if name=='bidiConformance' else 'cases'
    assert len(before[field])==len(after[field])
    if name not in ['pathRaster','sceneRaster','pageRaster']:
        assert before[field]==after[field],name;counts['unchangedBatches']+=len(after[field]);records[name]={'batches':len(after[field]),'casesUnchanged':True};continue
    frame_map={}
    for old,new in zip(before['cases'],after['cases'],strict=True):
        assert old['name']==new['name'] and old['status']==new['status'] and old['validRequest']==new['validRequest']
        if 'framePath' in new:
            raw=Path(new['framePath']).read_bytes();assert sha(raw)==new['frameSha256']
            restored=old_frame(raw);assert sha(restored)==old['frameSha256']
            assert restored==(ROOT/'previous/files'/old['framePath']).read_bytes()
            frame_map[new['frameSha256']]=old['frameSha256'];counts['exactOldFramesReconstructed']+=1
            assert new['pixelsSha256']==old['pixelsSha256']
            assert sha(Path(new['pixelsPath']).read_bytes())==old['pixelsSha256'];counts['identicalSolidPixels']+=1
    for old,new in zip(before['cases'],after['cases'],strict=True):
        for kind in ['request','response','plan']:
            if kind+'Path' not in old:continue
            old_bytes=(ROOT/'previous/files'/old[kind+'Path']).read_bytes();assert sha(old_bytes)==old[kind+'Sha256']
            new_bytes=Path(new[kind+'Path']).read_bytes();assert sha(new_bytes)==new[kind+'Sha256']
            try:a,b=json.loads(old_bytes),json.loads(new_bytes)
            except json.JSONDecodeError:assert old_bytes==new_bytes;continue
            assert a==restore(b,frame_map),(name,new['name'],kind)
            if kind=='plan':counts['plansChecked']+=1
        counts['migratedBatches']+=1
    records[name]={'batches':len(after['cases']),'typedBrushMigrationOnly':True,'allPixelsUnchanged':True,'oldFramesReconstructed':True}
assert counts['unchangedBatches']+counts['migratedBatches']==7680
result={'format':'musteroffice.gradient-raster-regression/1','previousEvidenceSha256':sha(BASE.read_bytes()),'counts':counts,'reports':records}
(ROOT/'regression-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(counts))
