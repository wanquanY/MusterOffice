"""80-digit source-definition oracle and source-identity checks for real presets.

Numeric expectations come from original ECMA XML plus eight explicit arity
corrections, not kernel geometry or generated catalog values. upArrow's paths,
handles and text bounds are checked against reflected downArrow geometry.
"""
import copy, hashlib, json, zipfile
from decimal import Decimal as D
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P,project
from geometry_reference import geometry
from geometry_eval_reference import evaluate,Unresolved
ROOT=Path('.codex-work/preset-expansion');NS={'a':A,'p':P}
sha=lambda b:hashlib.sha256(b).hexdigest()
source=Path('components/drawingml-presets/presetShapeDefinitions.xml').read_bytes()
assert sha(source)=='2f7c868d857c1e3c4b5a6068759fe0e07d77ad58377a6618d1b02ba3507b6939'
templates={}
for node in E.fromstring(source):templates.setdefault(node.tag,node)
manifest=json.loads((ROOT/'manifest.json').read_text());report=json.loads((ROOT/'parity.json').read_text());lookup={(c['name'],c['kind']):c for c in report['cases']}
catalog=json.loads(Path('components/drawingml-presets/catalog.json').read_text());blob=Path('components/drawingml-presets/catalog.xml').read_bytes();assert sha(blob)==catalog['catalogSha256']
values=origins=xsd_count=0;max_ratio=D(0);checks=[]
def compare(a,b,location):
    global values,max_ratio
    if isinstance(a,D):
        actual=D(str(b));difference=abs(a-actual);budget=D('1e-7')+D('2e-11')*max(abs(a),abs(actual))
        assert difference<=budget,(location,str(a),b,str(difference));max_ratio=max(max_ratio,difference/budget);values+=1
    elif isinstance(a,dict):
        assert a.keys()==b.keys(),(location,a.keys(),b.keys())
        for k in a:compare(a[k],b[k],location+'/'+k)
    elif isinstance(a,list):
        assert len(a)==len(b),location
        for i,(x,y) in enumerate(zip(a,b)):compare(x,y,location+'/'+str(i))
    else:assert a==b,(location,a,b)
def locate(v,preset):
    if isinstance(v,list):return [locate(x,preset) for x in v]
    if not isinstance(v,dict):return v
    if v.keys()=={'kind','sourceOrdinal'} and v['kind']=='document':
        n=v['sourceOrdinal'];return {'kind':'document','sourceOrdinal':-n-1} if n<0 else {'kind':'preset','preset':preset,'definitionOrdinal':n}
    return {k:locate(x,preset) for k,x in v.items()}
def unlocated(v):
    if isinstance(v,list):return [unlocated(x) for x in v]
    if not isinstance(v,dict):return v
    return {k:unlocated(x) for k,x in v.items() if k!='origin'}
def mirror(v,height):
    if isinstance(v,list):return [mirror(x,height) for x in v]
    if not isinstance(v,dict):return v
    return {k:height-x if k=='y' else mirror(x,height) for k,x in v.items()}
def validate_origins(v,nodes,document_nodes):
    global origins
    if isinstance(v,list):
        for x in v:validate_origins(x,nodes,document_nodes)
    elif isinstance(v,dict):
        if v.get('kind')=='preset' and 'definitionOrdinal' in v:
            assert 0<=v['definitionOrdinal']<len(nodes);origins+=1
        elif v.get('kind')=='document' and 'sourceOrdinal' in v:
            assert 0<=v['sourceOrdinal']<len(document_nodes);origins+=1
        else:
            for x in v.values():validate_origins(x,nodes,document_nodes)
schema=E.XMLSchema(E.parse('.codex-work/ecma376/xsd/pml.xsd',E.XMLParser(resolve_entities=False,no_network=True)))
for c in manifest['cases']:
    r=lookup[(c['name'],'geometry')];b=Path(r['responsePath']).read_bytes();assert sha(b)==r['responseSha256'];actual=json.loads(b)['geometry']['objects'][0]['outcome']
    deck=Path(c['path']).read_bytes();assert sha(deck)==c['sha256']
    with zipfile.ZipFile(c['path']) as z:xml=z.read('ppt/slides/slide1.xml')
    doc,_,ordinals=project(xml,with_ordinals=True);schema.assertValid(doc);xsd_count+=1
    shape=next(s for s in doc.findall('.//p:sp',NS) if int(s.find('p:nvSpPr/p:cNvPr',NS).get('id'))==c['objectId']);prst=shape.find('p:spPr/a:prstGeom',NS)
    declaration=geometry(prst,ordinals);preset=declaration['definition']['preset'];w,h=c['extent']
    t=copy.deepcopy(templates['downArrow' if preset=='upArrow' else preset]);t.tag=f'{{{A}}}custGeom'
    for gd in t.findall('.//a:gd',NS):
        f=gd.get('fmla');words=f.split()
        if len(words)==5:
            assert preset in ['circularArrow','leftCircularArrow','leftRightCircularArrow'] and words[0]=='+-' and words[-1]=='0'
            gd.set('fmla',' '.join(words[:4]))
    g=geometry(t,{n:i for i,n in enumerate(t.iter())});g['sourceOrdinal']=ordinals[prst]
    overrides=copy.deepcopy((declaration['definition']['adjustments'] or {'entries':[]})['entries'])
    for gd in overrides:gd['sourceOrdinal']=-gd['sourceOrdinal']-1
    if overrides:g['definition']['adjustments']['entries'].extend(overrides)
    try:expected=evaluate(g,w,h,{'wd12':D(w)/12,'wd32':D(w)/32,'hd10':D(h)/10,'cd3':D(7200000)})
    except Unresolved as e:
        assert actual['status']=='unresolved';compare(locate(e.reason,preset),actual['reason'],c['name']);checks.append({'name':c['name'],'status':'unresolved','responseSha256':r['responseSha256']});continue
    assert actual['status']=='resolved',c['name'];out=actual['geometry']
    expected['extent']={'value':{'width':str(w),'height':str(h)},'declaredBy':{'part':'/ppt/slides/slide1.xml','nativeId':c['objectId']}}
    if preset!='upArrow':compare(locate(expected,preset),out,c['name'])
    else:
        # Separate geometric construction: reflect evaluated downward-arrow
        # coordinates. It never reads the runtime upward-arrow formulas.
        compare(mirror(unlocated(expected['paths']),D(h)),unlocated(out['paths']),c['name']+'/reflectedPaths')
        rect=expected['textRect'];compare({'left':rect['left'],'top':D(h)-rect['bottom'],'right':rect['right'],'bottom':D(h)-rect['top']},unlocated(out['textRect']),c['name']+'/reflectedTextRect')
        for a,b in zip(expected['handles'],out['handles']):compare(mirror(unlocated(a['position']),D(h)),unlocated(b['position']),c['name']+'/reflectedHandle')
        for a,b in zip(expected['connections'],out['connections']):compare(mirror(unlocated(a['position']),D(h)),unlocated(b['position']),c['name']+'/reflectedConnection');compare(-a['angle'],b['angle'],c['name']+'/reflectedDirection')
    info=catalog['definitions'][preset];runtime=E.fromstring(blob[info['byteOffset']:info['byteOffset']+info['byteLength']]);nodes=list(runtime.iter());validate_origins(out,nodes,list(doc.iter()))
    for path in out['paths']:
        assert E.QName(nodes[path['origin']['definitionOrdinal']]).localname=='path'
        for cmd in path['commands']:
            tag={'move':'moveTo','line':'lnTo','quadratic':'quadBezTo','cubic':'cubicBezTo','close':'close','arc':'arcTo'}[cmd['kind']]
            assert E.QName(nodes[cmd['origin']['definitionOrdinal']]).localname==tag
    checks.append({'name':c['name'],'status':'resolved','responseSha256':r['responseSha256']})
result={'format':'musteroffice.preset-expansion-independent/1','cases':checks,'precisionDigits':80,'numericValues':values,'originsChecked':origins,'validXsdSlides':xsd_count,'maximumToleranceRatio':str(max_ratio),'absoluteTolerance':'1e-7','relativeTolerance':'2e-11','scope':'Independent source-based guide/coordinate checks and reflected upArrow geometry. Not Office/WPS visual or adjustment-handle interaction acceptance.'}
(ROOT/'independent.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='cases'}))
