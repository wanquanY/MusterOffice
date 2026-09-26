"""Owned presentation inputs for typed transform edits; independently XSD checked."""
import hashlib,json,re,zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/transform-edit');out=root/'sources';out.mkdir(parents=True,exist_ok=True)
seed=Path('.codex-work/pptx-mce-fixtures/authored.pptx')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
# Bind the frozen original digest, not a regenerated exporter output.
assert entry(seed)['sha256']=='779f732d9704e24f9f373f1e814d3e5759ac652ee46d5131187dfdc87978b93e'
with zipfile.ZipFile(seed) as z:base={n:z.read(n) for n in z.namelist()}
p='http://schemas.openxmlformats.org/presentationml/2006/main';a='http://schemas.openxmlformats.org/drawingml/2006/main'
mc='http://schemas.openxmlformats.org/markup-compatibility/2006';ns={'p':p,'a':a}
parser=X.XMLParser(resolve_entities=False,no_network=True);schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd',parser))
slide='ppt/slides/slide1.xml';layout='ppt/slideLayouts/slideLayout2.xml'
records=[];parts=0
def store(name,data,valid=True):
    global parts
    checked=0
    if valid:
        for n,b in data.items():
            if not n.endswith('.xml'):continue
            e=X.fromstring(b,parser)
            if X.QName(e).namespace==p and X.QName(e).localname in ['presentation','sld','sldLayout','sldMaster']:
                schema.assertValid(e);checked+=1
    parts+=checked;path=out/(name+'.pptx')
    with zipfile.ZipFile(path,'w') as z:
        for n,b in data.items():
            i=zipfile.ZipInfo(n,(2026,9,26,0,0,0));i.compress_type=zipfile.ZIP_DEFLATED;z.writestr(i,b)
    records.append(dict(name=name,source=entry(path),officialXsdParts=checked,rawXsd=valid))
def modify(name,fn,valid=True):
    data=dict(base);data[slide]=fn(data[slide].decode()).encode();store(name,data,valid)
tag='<a:xfrm rot="0" flipH="0" flipV="0">'
assert tag in base[slide].decode()
store('base',base)
modify('root',lambda s:s.replace('<p:grpSpPr/>','<p:grpSpPr><a:xfrm rot="0"><a:off x="0" y="0"/><a:ext cx="12000000" cy="6750000"/><a:chOff x="0" y="0"/><a:chExt cx="12000000" cy="6750000"/></a:xfrm></p:grpSpPr>',1))
modify('absent-attributes',lambda s:s.replace(tag,'<a:xfrm>',1))
modify('missing-transform',lambda s:re.sub(r'<a:xfrm[^>]*>.*?</a:xfrm>','',s,count=1))
modify('missing-origin',lambda s:re.sub(r'<a:off[^>]*/>','',s,count=1))
modify('unknown-attribute',lambda s:s.replace(tag,'<a:xfrm xmlns:q="urn:owned-future" q:cached="yes" rot="0" flipH="0" flipV="0">',1),False)
modify('ignored-attribute',lambda s:s.replace(tag,f'<a:xfrm xmlns:mc="{mc}" xmlns:q="urn:owned-future" mc:Ignorable="q" q:cached="yes" rot="0" flipH="0" flipV="0">',1),False)
modify('ignored-child',lambda s:s.replace(tag,f'<a:xfrm xmlns:mc="{mc}" xmlns:q="urn:owned-future" mc:Ignorable="q" rot="0" flipH="0" flipV="0"><q:cache/>',1),False)
def alternate(s):
    first=s.index('<p:sp>');last=s.index('</p:sp>',first)+7;shape=s[first:last]
    return s[:first]+f'<mc:AlternateContent xmlns:mc="{mc}" xmlns:q="urn:owned-future"><mc:Choice Requires="q">{shape}</mc:Choice><mc:Fallback>{shape}</mc:Fallback></mc:AlternateContent>'+s[last:]
modify('alternate-object',alternate,False)
data=dict(base)
for path in [slide,layout]:
    text=data[path].decode().replace('<p:cNvSpPr/><p:nvPr/>','<p:cNvSpPr/><p:nvPr><p:ph idx="7"/></p:nvPr>',1)
    if path==slide:text=re.sub(r'<a:xfrm[^>]*>.*?</a:xfrm>','',text,count=1)
    data[path]=text.encode()
store('inherited',data)
for enc in ['utf-8','utf-16le','utf-16be']:
    for bom in [False,True]:
        text=base[slide].decode().replace('encoding="UTF-8"','encoding="'+('UTF-8' if enc=='utf-8' else 'UTF-16')+'"')
        text=text.replace(tag,"<a:xfrm rot = '+000' flipH='false' flipV = '0' >",1)
        text=text.replace('<p:cSld','<!-- 保留原字节 🚀 --><?owned preserve?><p:cSld',1)
        text=text.replace('</p:sld>','<p:extLst><p:ext uri="urn:owned"><q:keep xmlns:q="urn:owned"><![CDATA[<a:xfrm bogus="1"/>]]></q:keep></p:ext></p:extLst></p:sld>')
        prefix={'utf-8':b'\xef\xbb\xbf','utf-16le':b'\xff\xfe','utf-16be':b'\xfe\xff'}[enc] if bom else b''
        data=dict(base);data[slide]=prefix+text.encode(enc);store(enc+('-bom' if bom else '-bare'),data)
# Native graphic frame with editable table content, and a native connector.
extras=f'''<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="900" name="owned-table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="100" y="200"/><a:ext cx="300" cy="400"/></p:xfrm><a:graphic><a:graphicData uri="{a}/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="300"/></a:tblGrid><a:tr h="400"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame><p:cxnSp><p:nvCxnSpPr><p:cNvPr id="901" name="owned-connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="300" cy="400"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>'''
modify('frame-connector',lambda s:s.replace('</p:spTree>',extras+'</p:spTree>'))
(root/'fixtures.json').write_text(json.dumps(dict(format='musteroffice.transform-edit-inputs/1',seed=entry(seed),cases=records,officialXsdParts=parts),indent=2)+'\n')
print(json.dumps(dict(fixtures=len(records),officialXsdParts=parts)))
