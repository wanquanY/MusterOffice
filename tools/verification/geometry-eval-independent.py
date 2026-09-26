"""Independent high-precision formula/path checks on the real native/WASM corpus."""
import hashlib
import json
from decimal import Decimal as D
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import A,P,project
from geometry_reference import geometry
from geometry_eval_reference import evaluate,Unresolved

ROOT=Path('.codex-work/geometry-eval');NS={'a':A,'p':P}
manifest=json.loads((ROOT/'manifest.json').read_text());report=json.loads((ROOT/'parity.json').read_text());lookup={c['name']:c for c in report['cases']}
ABS_TOL=D('1e-7');REL_TOL=D('2e-11')
max_abs=D(0);max_rel=D(0);max_budget_ratio=D(0);max_abs_location=None;max_rel_location=None
values=resolved=unresolved=0;checks=[];operations=set();reasons={};xsd_count=edited=0
XSD=Path('.codex-work/ecma376/xsd')
schema=E.XMLSchema(E.parse(str(XSD/'pml.xsd'),E.XMLParser(resolve_entities=False,no_network=True)))
def sha(b):return hashlib.sha256(b).hexdigest()
def compare(a,b,location):
    global max_abs,max_rel,max_budget_ratio,max_abs_location,max_rel_location,values
    if isinstance(a,D):
        actual=D(str(b));diff=abs(a-actual);scale=max(abs(a),abs(actual));relative=diff/scale if scale else D(0)
        assert diff<=ABS_TOL+REL_TOL*scale,(location,str(a),b,str(diff))
        if diff>max_abs:max_abs=diff;max_abs_location={'path':location,'expected':str(a),'actual':b}
        if relative>max_rel:max_rel=relative;max_rel_location={'path':location,'expected':str(a),'actual':b}
        max_budget_ratio=max(max_budget_ratio,diff/(ABS_TOL+REL_TOL*scale));values+=1
    elif isinstance(a,dict):
        assert a.keys()==b.keys(),(location,a.keys(),b.keys())
        for k in a:compare(a[k],b[k],location+'/'+k)
    elif isinstance(a,list):
        assert len(a)==len(b),location
        for i,(x,y) in enumerate(zip(a,b)):compare(x,y,location+'/'+str(i))
    else:assert a==b,(location,a,b)
for c in manifest['cases']:
    r=lookup[c['name']];raw=Path(c['path']).read_bytes();response=Path(r['responsePath']).read_bytes()
    assert sha(raw)==c['sha256'] and sha(response)==r['responseSha256']
    result=json.loads(response)['geometry']['objects'][0]['outcome']
    with zipfile.ZipFile(c['path']) as z:parts={n:z.read(n) for n in z.namelist()}
    for part,raw in parts.items():
        if not any(part.startswith(p) for p in ['ppt/slides/slide','ppt/slideLayouts/slideLayout','ppt/slideMasters/slideMaster']) or not part.endswith('.xml'):continue
        if c['name']=='source-retained-attribute' and '/'+part==c['part']:continue
        schema.assertValid(project(raw)[0]);xsd_count+=1
    tree,_,ordinals=project(parts[c['part'].lstrip('/')],with_ordinals=True)
    shape=next(s for s in tree.findall('.//p:sp',NS) if int(s.find('p:nvSpPr/p:cNvPr',NS).get('id'))==c['objectId'])
    props=shape.find('p:spPr',NS);extent=props.find('a:xfrm/a:ext',NS)
    g=geometry(props.find('a:custGeom',NS),ordinals)
    for key in ['adjustments','guides']:
        for guide in (g['definition'][key] or {'entries':[]})['entries']:
            if guide['formula'].split():operations.add(guide['formula'].split()[0])
    try:
        expected=evaluate(g,int(extent.get('cx')),int(extent.get('cy')))
        expected['extent']={'value':{'width':extent.get('cx'),'height':extent.get('cy')},'declaredBy':{'part':c['part'],'nativeId':c['objectId']}}
    except Unresolved as error:
        assert result['status']=='unresolved',(c['name'],error.reason,result)
        assert error.reason==result['reason'],(c['name'],error.reason,result['reason'])
        key=error.reason.get('issue',error.reason['kind']);reasons[key]=reasons.get(key,0)+1;unresolved+=1
    else:
        assert result['status']=='resolved',(c['name'],result)
        compare(expected,result['geometry'],c['name']);resolved+=1
    if 'edited-'+c['name'] in lookup:
        e=lookup['edited-'+c['name']];assert sha(Path(e['sourcePath']).read_bytes())==e['sourceSha256']
        with zipfile.ZipFile(e['sourcePath']) as z:after={n:z.read(n) for n in z.namelist()}
        assert parts.keys()==after.keys()
        for part,raw in parts.items():
            if '/'+part!=c['part']:assert raw==after[part]
        before_tree=E.fromstring(parts[c['part'].lstrip('/')]);after_tree=E.fromstring(after[c['part'].lstrip('/')])
        path='p:cSld/p:spTree/p:sp/p:txBody/a:p/a:r/a:t'
        a=before_tree.find(path,NS);b=after_tree.find(path,NS);assert b.text=='geometry evaluated 中文';b.text=a.text
        assert E.tostring(before_tree)==E.tostring(after_tree),c['name'];edited+=1
    checks.append({'name':c['name'],'sourceSha256':c['sha256'],'responseSha256':r['responseSha256'],'outcome':result['status']})
result={'format':'musteroffice.geometry-evaluation-independent/1','cases':checks,'resolved':resolved,'unresolved':unresolved,'reasons':reasons,
        'numericValues':values,'formulaTokens':sorted(operations),'absoluteTolerance':str(ABS_TOL),'relativeTolerance':str(REL_TOL),
        'maxAbsoluteDifference':str(max_abs),'maxRelativeDifference':str(max_rel),'maxToleranceRatio':str(max_budget_ratio),
        'maxAbsoluteLocation':max_abs_location,'maxRelativeLocation':max_rel_location,'precisionDigits':80,
        'validXsdParts':xsd_count,'editedPackagesPreserved':edited,'intentionalXsdExclusions':['source-retained-attribute'],
        'xsdInputs':[{'path':str(p),'sha256':sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))],
        'scope':'Independent original-XML guide and coordinate arithmetic. No preset runtime expansion, arc tessellation, rendering, or Office/WPS acceptance.'}
(ROOT/'independent.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['cases','scope','xsdInputs']}))
