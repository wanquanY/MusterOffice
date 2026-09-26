"""Independent Decimal author-space point evaluator; no Rust interval/matrix reuse."""
import hashlib
import json
import sys
from decimal import Decimal as D, getcontext
from functools import lru_cache
from pathlib import Path

getcontext().prec=140
ROOT=Path(sys.argv[1] if len(sys.argv)>1 else '.codex-work/page-placement');U=D(2)**32
def read(p):return json.loads(Path(p).read_text())
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
from page_reference_math import cs, size, dims, resolved
counts={'documents':0,'objects':0,'coefficients':0,'sourceControlPoints':0};records=[]
for case in read(ROOT/'parity.json')['cases']:
    if case['status']!='evaluated':continue
    q=read(case['requestPath']);result=read(case['responsePath'])['result'];d=q['document'];counts['documents']+=1
    assert result['slide']==q['slide'] and result['pageSize']==d['pageSize']
    slide=d['slides'][q['slide']];roots=[]
    if slide['layout']:
        layout=d['layouts'][slide['layout']];master=d['masters'][layout['master']]
        roots += [('master',master),('layout',layout)]
    roots += [('slide',slide)]
    assert len(roots)==len(result['surfaces'])
    used=[]
    for (kind,surface),out in zip(roots,result['surfaces'],strict=True):
        assert out['container']=={'kind':kind,'id':surface['id']}
        expected=[];pending=list(reversed(surface['objects']))
        while pending:
            id=pending.pop();expected.append(id);o=d['objects'][id]
            if o['content']['kind']=='group':pending.extend(reversed(o['content']['children']))
        assert [o['object'] for o in out['objects']]==expected
        for p in out['objects']:
            o=d['objects'][p['object']];chain=[];parent=o['parent']
            while parent['kind']=='group':
                g=d['objects'][parent['id']];chain.append(g);parent=g['parent']
            assert p['depth']==len(chain) and p['parent']==o['parent'] and p['sourceSize']==size(o)
            src=dims(size(o));anchor=[D(p['anchor'][a])/U for a in ['x','y']];assert anchor==[v/2 for v in src]
            center,scale,angle=resolved(list(reversed(chain))+[o])
            c,s=cs(angle);lin=[c*scale[0],-s*scale[1],s*scale[0],c*scale[1]]
            actual=[D(v)/U for v in p['affine']['linear']]+[D(p['affine']['translation'][a])/U for a in ['x','y']]
            errors=[D(v)/U for v in p['uncertainty']['linear']]+[D(p['uncertainty']['translation'][a])/U for a in ['x','y']]
            for a,b,e in zip(actual,lin+center,errors,strict=True):assert e>=0 and abs(a-b)<=e+D('1e-95'),(case['name'],o['id'],a,b,e)
            points=[[D(0),D(0)],[src[0],D(0)],src,[D(0),src[1]],anchor]
            if o['content']['kind']=='shape' and o['content']['geometry']['kind']=='path':
                for cmd in o['content']['geometry']['commands']:
                    points += [[D(v['x']),D(v['y'])] for k,v in cmd.items() if k!='kind']
            for point in points:
                delta=[point[i]-anchor[i] for i in range(2)]
                for row in range(2):
                    got=sum(actual[2*row+i]*delta[i] for i in range(2))+actual[4+row]
                    reference=sum(lin[2*row+i]*delta[i] for i in range(2))+center[row]
                    bound=sum(errors[2*row+i]*abs(delta[i]) for i in range(2))+errors[4+row]
                    assert abs(got-reference)<=bound+D('1e-90')
            counts['objects']+=1;counts['coefficients']+=6;counts['sourceControlPoints']+=len(points);used.append(p['object'])
    records.append({'name':case['name'],'responseSha256':sha(case['responsePath']),'objects':len(used)})
report={'format':'musteroffice.page-placement-reference/1','arithmetic':'Decimal 140 digits; Chudnovsky pi; root-to-leaf resolved extents, reflected angles and rotation sectors; 1e-90 or smaller reference guard','counts':counts,'cases':records}
(ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(counts))
