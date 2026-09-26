"""Verify raw transform records, retained physical ordinals and valid native XML."""
import hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P
ROOT=Path('.codex-work/source-placement');N={'a':A,'p':P}
read=lambda p:json.loads(Path(p).read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
schema=E.XMLSchema(E.parse('.codex-work/ecma376/xsd/pml.xsd',E.XMLParser(resolve_entities=False,no_network=True)))
transforms=valid=retained=0;invalid=[];cases=[]
def check(xml,t,r):
    global transforms,retained
    if t is None:assert r is None;return
    transforms+=1;expect={}
    for x,key,attrs in [('off','origin',['x','y']),('ext','size',['cx','cy']),('chOff','childOrigin',['x','y']),('chExt','childSize',['cx','cy'])]:
        e=t.find('a:'+x,N)
        expect[key]=None if e is None else dict(zip(['x','y'] if x.endswith('Off') or x=='off' else ['width','height'],[str(int(e.get(a))) for a in attrs],strict=True))
    for a,key in [('rot','rotation'),('flipH','flipHorizontal'),('flipV','flipVertical')]:expect[key]=None if a not in t.attrib else int(t.get(a)) if a=='rot' else t.get(a) in ['1','true']
    assert {k:v for k,v in r.items() if k!='retainedOrdinals'}==expect
    nodes=list(xml.iter());ordinal={id(n):i for i,n in enumerate(nodes)}
    unknown=[]
    if set(t.attrib)-{'rot','flipH','flipV'}:unknown.append(ordinal[id(t)])
    for n in t:
        if E.QName(n).namespace!=A or E.QName(n).localname not in ['off','ext','chOff','chExt']:unknown.append(ordinal[id(n)])
    assert r.get('retainedOrdinals',[])==unknown;retained+=len(unknown)
for c in read(ROOT/'manifest.json')['cases']:
    b=Path(c['path']).read_bytes();assert sha(b)==c['sha256'];r=read(ROOT/(c['name']+'.source.response.json'))
    with zipfile.ZipFile(c['path']) as z:
        slide=E.fromstring(z.read('ppt/slides/slide1.xml'))
        if schema.validate(slide):valid+=1
        else:
            invalid.append({'name':c['name'],'reason':str(schema.error_log.last_error)})
            assert c['error'] or c['unresolved'] in ['retainedTransform','invalidCoordinate'],c['name']
        if c['error']:assert r['status']=='error';continue
        for part,surface in r['index']['surfaces'].items():
            doc=E.fromstring(z.read(part[1:]));st=doc.find('p:cSld/p:spTree',N)
            check(doc,st.find('p:grpSpPr/a:xfrm',N),surface.get('rootGroupTransform'))
            objects={}
            for n in st.iter():
                if E.QName(n).namespace==P and E.QName(n).localname in ['sp','pic','cxnSp','grpSp','graphicFrame']:
                    ident=n.find('./*/p:cNvPr',N);objects[int(ident.get('id'))]=n
            for o in surface['objects']:
                n=objects[o['nativeId']];t=n.find('p:grpSpPr/a:xfrm' if o['kind']=='group' else 'p:xfrm' if o['kind']=='graphicFrame' else 'p:spPr/a:xfrm',N)
                check(doc,t,o['transform'])
        cases.append({'name':c['name'],'sourceSha256':c['sha256'],'responseSha256':sha((ROOT/(c['name']+'.source.response.json')).read_bytes())})
result={'format':'musteroffice.source-placement-source-check/1','transforms':transforms,'validXsdSlides':valid,'invalidXsdSlides':invalid,'retainedLocations':retained,'cases':cases}
(ROOT/'source-check.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='cases'}))
