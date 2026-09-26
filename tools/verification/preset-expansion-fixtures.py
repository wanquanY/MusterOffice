"""Real prstGeom packages with independently authored extents and overrides."""
import copy, hashlib, json, zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A, P
ROOT=Path('.codex-work/preset-expansion');ROOT.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
original=json.loads(Path('.codex-work/source-geometry/manifest.json').read_text())
bases={c['name'][7:]:c for c in original['cases'] if c['name'].startswith('preset-') and c['name']!='preset-token-space'}
assert len(bases)==187
cases=[]
NS={'a':A,'p':P}
def emit(preset,label,w,h,adjustments=(),error=None):
    base=bases[preset];assert sha(Path(base['path']).read_bytes())==base['sha256']
    with zipfile.ZipFile(base['path']) as z:parts={n:z.read(n) for n in z.namelist()}
    root=E.fromstring(parts['ppt/slides/slide1.xml']);shape=root.find('p:cSld/p:spTree/p:sp',NS)
    props=shape.find('p:spPr',NS)
    xfrm=E.Element(f'{{{A}}}xfrm');E.SubElement(xfrm,f'{{{A}}}off',x='0',y='0');E.SubElement(xfrm,f'{{{A}}}ext',cx=str(w),cy=str(h));props.insert(0,xfrm)
    g=props.find('a:prstGeom',NS)
    if adjustments:
        av=E.SubElement(g,f'{{{A}}}avLst')
        for name,formula in adjustments:E.SubElement(av,f'{{{A}}}gd',name=name,fmla=formula)
    parts['ppt/slides/slide1.xml']=E.tostring(root,encoding='UTF-8',xml_declaration=True)
    name=preset+'-'+label;path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w') as z:
        for n,b in parts.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    cases.append({'name':name,'preset':preset,'path':str(path),'sha256':sha(path.read_bytes()),'objectId':int(shape.find('p:nvSpPr/p:cNvPr',NS).get('id')),'extent':[w,h],'adjustments':adjustments,'error':error,'render':label=='square'})
for preset in sorted(bases):
    for label,w,h in [('square',1000001,1000001),('wide',2000003,700001),('tall',700001,2000003)]:emit(preset,label,w,h)
for preset,values in [('triangle',[('adj','val 25000')]),('roundRect',[('adj','val 0')]),('upArrow',[('adj1','val 25000'),('adj2','val 30000')]),('star5',[('adj','val 12000')]),('pie',[('adj1','val 2700000'),('adj2','val 16200000')]),('circularArrow',[('adj1','val 10000')]),('triangle',[('adj','val 25000'),('adj','+- adj 10000 0')])]:
    emit(preset,'override-'+str(len(cases)),1000001,700001,values)
for label,values,kind in [('unknown',[('adj','val missing')],'unknownReference'),('invalid',[('adj','val 1 2')],'formula'),('reserved',[('w','val 1')],'reservedGuide')]:emit('triangle','reject-'+label,1000001,700001,values,kind)
(ROOT/'manifest.json').write_text(json.dumps({'cases':cases},indent=2)+'\n');print(json.dumps({'cases':len(cases),'presets':len(bases),'renderedCandidates':sum(c['render'] for c in cases)}))
