"""Independent XML/ZIP/XSD verification of actual public transform-edit outputs."""
import hashlib,json,re,struct,zipfile
from pathlib import Path
from xml.parsers import expat
from lxml import etree as X
root=Path('.codex-work/transform-edit');report=json.loads((root/'product.json').read_text())
ns={'a':'http://schemas.openxmlformats.org/drawingml/2006/main','p':'http://schemas.openxmlformats.org/presentationml/2006/main'}
parser=X.XMLParser(resolve_entities=False,no_network=True);schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd',parser))
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(r):
    assert entry(r['path'])==r;return Path(r['path']).read_bytes()
def decode(b):
    for mark,enc in [(b'\xef\xbb\xbf','utf-8'),(b'\xff\xfe','utf-16le'),(b'\xfe\xff','utf-16be')]:
        if b.startswith(mark):return b[len(mark):].decode(enc),mark,enc
    enc='utf-16le' if b[:4]==b'<\0?\0' else 'utf-16be' if b[:4]==b'\0<\0?' else 'utf-8'
    return b.decode(enc),b'',enc
def compressed(path,info):
    data=Path(path).read_bytes();start=info.header_offset
    assert data[start:start+4]==b'PK\x03\x04'
    names,extra=struct.unpack_from('<HH',data,start+26);at=start+30+names+extra
    return data[at:at+info.compress_size]
def pairs(v,local):
    if local=='xfrm':return {'rot':None if v['rotation'] is None else str(v['rotation']),
        'flipH':None if v['flipHorizontal'] is None else str(int(v['flipHorizontal'])),
        'flipV':None if v['flipVertical'] is None else str(int(v['flipVertical']))}
    field={'off':'origin','ext':'size','chOff':'childOrigin','chExt':'childSize'}[local];q=v[field]
    if local in ['off','chOff']:return {'x':q['x'],'y':q['y']}
    return {'cx':q['width'],'cy':q['height']}
def observed(node):
    def point(local):
        c=node.find('a:'+local,ns);return None if c is None else {'x':str(int(c.get('x'))),'y':str(int(c.get('y')))}
    def size(local):
        c=node.find('a:'+local,ns);return None if c is None else {'width':str(int(c.get('cx'))),'height':str(int(c.get('cy')))}
    def boolean(name):
        s=node.get(name);return None if s is None else {'0':False,'1':True,'false':False,'true':True}[s.strip()]
    return dict(origin=point('off'),size=size('ext'),childOrigin=point('chOff'),childSize=size('chExt'),
        rotation=None if node.get('rot') is None else int(node.get('rot')),flipHorizontal=boolean('flipH'),flipVertical=boolean('flipV'))
token=re.compile(rb'<(?:[^\'">]|\'[^\']*\'|"[^"]*")*>',re.S)
attribute=re.compile(rb'([^\s=<>/\'\"]+)\s*=\s*([\'\"])(.*?)\2',re.S)
def expected_xml(original,edits):
    text,bom,encoding=decode(original);raw=text.encode();tree=X.fromstring(original,parser)
    nodes=[n for n in tree.iter() if isinstance(n.tag,str)];positions=[]
    scan=expat.ParserCreate(encoding='UTF-8');scan.StartElementHandler=lambda name,attrs:positions.append(scan.CurrentByteIndex)
    scan.Parse(raw,True);assert len(nodes)==len(positions)
    offsets=dict(zip(nodes,positions));patches=[]
    for edit in edits:
        old,new=edit['expected'],edit['replacement']
        if old==new:continue
        candidates=tree.xpath('.//p:cNvPr[@id="'+str(edit['target']['nativeId'])+'"]',namespaces=ns)
        assert len(candidates)==1
        owner=candidates[0].getparent().getparent();local=X.QName(owner).localname
        path='p:xfrm' if local=='graphicFrame' else 'p:grpSpPr/a:xfrm' if local in ['grpSp','spTree'] else 'p:spPr/a:xfrm'
        transform=owner.find(path,ns);assert transform is not None;assert observed(transform)==old
        for node in [transform,*list(transform)]:
            if not isinstance(node.tag,str):continue
            name=X.QName(node).localname;before=pairs(old,name);after=pairs(new,name)
            start=offsets[node];m=token.match(raw,start);assert m
            tag=m.group();attrs={m[1].decode():m for m in attribute.finditer(tag)};insert=[]
            for key in sorted(before):
                if before[key]==after[key]:continue
                value=after[key]
                if key in attrs:
                    a=attrs[key]
                    if value is None:patches.append((start+a.start(1),start+a.end(),b''));del node.attrib[key]
                    else:patches.append((start+a.start(3),start+a.end(3),value.encode()));node.set(key,value)
                else:
                    assert value is not None;insert.append(b' '+key.encode()+b'="'+value.encode()+b'"');node.set(key,value)
            if insert:
                at=m.end()-(2 if tag.endswith(b'/>') else 1);patches.append((at,at,b''.join(insert)))
    end=-1
    for start,finish,value in sorted(patches):assert start>=end;end=finish
    for start,finish,value in sorted(patches,reverse=True):raw=raw[:start]+value+raw[finish:]
    return bom+raw.decode().encode(encoding),tree

records=[];xsd_parts=preserved_parts=preserved_compressed=changed_xml=0
for case in report['cases']:
    if case['status']!='edited':continue
    load(case['source']);load(case['output']);request=json.loads(load(case['request']))
    edits={}
    for e in request['edits']:edits.setdefault(e['target']['part'].lstrip('/'),[]).append(e)
    with zipfile.ZipFile(case['source']['path']) as before,zipfile.ZipFile(case['output']['path']) as after:
        assert before.namelist()==after.namelist();changed=[]
        for info in before.infolist():
            name=info.filename;source=before.read(name);actual=after.read(name)
            if name in edits:
                expected,tree=expected_xml(source,edits[name]);assert actual==expected,(case['name'],name,'lexical bytes')
                parsed=X.fromstring(actual,parser)
                assert X.tostring(parsed,method='c14n')==X.tostring(tree,method='c14n'),(case['name'],name,'semantics')
            else:assert actual==source,(case['name'],name,'untouched part')
            if actual==source:
                preserved_parts+=1;assert compressed(case['source']['path'],info)==compressed(case['output']['path'],after.getinfo(name));preserved_compressed+=1
            else:changed.append(name);changed_xml+=1
            if case['rawXsd'] and name.endswith('.xml'):
                node=X.fromstring(actual,parser);q=X.QName(node)
                if q.namespace==ns['p'] and q.localname in ['presentation','sld','sldLayout','sldMaster']:
                    schema.assertValid(node);xsd_parts+=1
        records.append(dict(name=case['name'],source=case['source'],output=case['output'],changedParts=changed))
assert len(records)==report['edited']
result=dict(format='musteroffice.transform-edit-reference/1',cases=records,independentParser='lxml/libxml2 + Expat',
    libxmlVersion=X.LIBXML_VERSION,expatVersion=expat.EXPAT_VERSION,sourceReport=entry(root/'product.json'),
    lexicalAndSemanticCases=len(records),changedXmlParts=changed_xml,preservedParts=preserved_parts,
    preservedCompressedEntries=preserved_compressed,officialXsdParts=xsd_parts,
    limits='Raw XSD excludes intentionally retained unknown/MCE input probes; no Office/WPS application acceptance.')
(root/'reference.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['cases','sourceReport']}))
