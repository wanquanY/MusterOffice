"""Owned native circle gradients to distinguish scalar algorithms in target apps."""
import hashlib,json,zipfile
from pathlib import Path
from lxml import etree as X

root=Path('.codex-work/radial-observation');out=root/'sources';out.mkdir(parents=True,exist_ok=True)
seed=Path('.codex-work/radial-layout/native/center-point.pptx')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
manifest=json.loads(Path('.codex-work/radial-layout/reference.json').read_text())
assert entry(seed) in manifest['sourceFiles']
ns=dict(a='http://schemas.openxmlformats.org/drawingml/2006/main',p='http://schemas.openxmlformats.org/presentationml/2006/main')
parser=X.XMLParser(resolve_entities=False,no_network=True)
xsd=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd',parser))
cases=[
    ('center-point',['50%','50%','50%','50%']),
    ('offset-point',['80%','20%','20%','80%']),
    ('external-point',['150%','50%','-50%','50%']),
    ('center-ellipse',['35%','10%','35%','10%']),
    ('offset-ellipse',['10%','25%','60%','15%']),
    ('external-ellipse',['80%','40%','-20%','40%']),
    ('equal-width-offset',['10%','25%','-10%','25%']),
    ('near-equal-width',['10%','25%','-9.999999999999999999%','25%']),
    ('center-line',['50%','20%','50%','20%']),
    ('whole-circle',['0','0','0','0']),
    ('expanded-focus',['-50%','-25%','-50%','-25%']),
]
records=[];parts=0
for name,margins in cases:
    with zipfile.ZipFile(seed) as z:data={n:z.read(n) for n in z.namelist()}
    page=X.fromstring(data['ppt/slides/slide1.xml'],parser)
    sp=page.find('p:cSld/p:spTree/p:sp',ns)
    sp.find('p:nvSpPr/p:cNvPr',ns).set('name','Owned radial observation '+name)
    stops=sp.find('p:spPr/a:gradFill/a:gsLst',ns);stops.clear()
    for pos,color in [('0','202020'),('50000','808080'),('100000','E0E0E0')]:
        stop=X.SubElement(stops,'{'+ns['a']+'}gs',pos=pos);X.SubElement(stop,'{'+ns['a']+'}srgbClr',val=color)
    focus=sp.find('p:spPr/a:gradFill/a:path/a:fillToRect',ns)
    for k,v in zip(['l','t','r','b'],margins):focus.set(k,v)
    tree=page.find('p:cSld/p:spTree',ns)
    for i,(x,y) in enumerate([(50000,50000),(1500000,50000),(50000,1100000),(1500000,1100000)]):
        marker=f'<p:sp xmlns:p="{ns["p"]}" xmlns:a="{ns["a"]}"><p:nvSpPr><p:cNvPr id="{100+i}" name="Owned registration {i}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="20000" cy="20000"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="FF00FF"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>'
        tree.append(X.fromstring(marker.encode(),parser))
    data['ppt/slides/slide1.xml']=X.tostring(page,xml_declaration=True,encoding='UTF-8',standalone=True)
    for n,b in data.items():
        if n.endswith('.xml'):
            e=X.fromstring(b,parser);q=X.QName(e)
            if q.namespace==ns['p'] and q.localname in ['presentation','sld','sldLayout','sldMaster']:xsd.assertValid(e);parts+=1
    p=out/('radial-'+name+'.pptx')
    with zipfile.ZipFile(p,'w',compression=zipfile.ZIP_DEFLATED) as z:
        for n,b in data.items():
            info=zipfile.ZipInfo(n,(2026,9,25,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    records.append(dict(name=name,margins=margins,source=entry(p)))
report=dict(format='musteroffice.radial-observation-inputs/1',seed=entry(seed),officialXsdParts=parts,cases=records,
            scope='Owned native grayscale three-stop ramps; distinct endpoint colors prevent Office special gamma interpolation. Four native magenta registration markers; no screenshots or flattened objects in PPTX.')
(root/'sources.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(cases=len(records),parts=parts)))
