"""Owned native arc/path boundary inputs plus the existing real formula corpus."""
import hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P
ROOT=Path('.codex-work/native-paths');ROOT.mkdir(exist_ok=True)
NS={'a':A,'p':P};sha=lambda b:hashlib.sha256(b).hexdigest()
cases=json.loads(Path('.codex-work/geometry-eval/manifest.json').read_text())['cases']
base=next(c for c in json.loads(Path('.codex-work/source-geometry/manifest.json').read_text())['cases'] if c['name']=='empty-path')
assert sha(Path(base['path']).read_bytes())==base['sha256']
with zipfile.ZipFile(base['path']) as z:original={n:z.read(n) for n in z.namelist()}
def emit(name,commands,guides='',dims='',expected=None,tolerance='4294967296'):
    parts=dict(original);part=base['part'].lstrip('/');root=E.fromstring(parts[part]);shape=root.find('p:cSld/p:spTree/p:sp',NS);props=shape.find('p:spPr',NS)
    for child in list(props):props.remove(child)
    props.append(E.fromstring(f'<a:xfrm xmlns:a="{A}"><a:off x="0" y="0"/><a:ext cx="2160000" cy="1080000"/></a:xfrm>'))
    props.append(E.fromstring(f'<a:custGeom xmlns:a="{A}"><a:gdLst>{guides}</a:gdLst><a:pathLst><a:path {dims}>{commands}</a:path></a:pathLst></a:custGeom>'))
    parts[part]=E.tostring(root,xml_declaration=True,encoding='UTF-8');path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w') as z:
        for n,b in parts.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    cases.append({'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'part':base['part'],'objectId':int(shape.find('p:nvSpPr/p:cNvPr',NS).get('id')),'pathExpected':expected,'tolerance':tolerance,'owned':True})
move='<a:moveTo><a:pt x="900000" y="400000"/></a:moveTo>'
for index,(rx,ry) in enumerate([(900000,300000),(300000,900000),(500000,500000),(1,5000000)]):
    for angle in [-21600001,-2700000,-0.125,0,1,2700000,5400000,10800001,16200000,21600001]:
        for sweep in [-43200000,-8100000,0,5400000,27000000]:
            guides=''.join(f'<a:gd name="{name}" fmla="val {value}"/>' for name,value in [('rx',rx),('ry',ry),('start',angle),('sweep',sweep)])
            emit(f'arc-{index}-{angle}-{sweep}',move+'<a:arcTo wR="rx" hR="ry" stAng="start" swAng="sweep"/>',guides,expected='compiled')
for name,attrs,expected in [('zero-radius','wR="0" hR="100"','degenerateArcRadius'),('negative-radius','wR="-2" hR="100"','negativeRadius')]:
    emit(name,move+f'<a:arcTo {attrs} stAng="0" swAng="5400000"/>',expected=expected)
for name,dims,expected in [('zero-width','w="0" h="100"','zeroPathExtent'),('scale-axes','w="3000000" h="5000000"','compiled'),('scale-width','w="3000000"','compiled')]:
    emit(name,move+'<a:quadBezTo><a:pt x="-200000" y="0"/><a:pt x="3000000" y="5000000"/></a:quadBezTo><a:close/><a:lnTo><a:pt x="100000" y="200000"/></a:lnTo>',dims=dims,expected=expected)
emit('implicit-start','<a:lnTo><a:pt x="100" y="200"/></a:lnTo><a:close/><a:close/>',expected='compiled')
(ROOT/'manifest.json').write_text(json.dumps({'cases':cases,'priorPptxInputs':602,'ownedInputs':len(cases)-602},indent=2)+'\n')
print(json.dumps({'cases':len(cases),'owned':len(cases)-602}))
