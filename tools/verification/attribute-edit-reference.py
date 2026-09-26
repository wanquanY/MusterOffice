"""Independent XML parser plus exact lexical expectations and Native/WASM probes."""
import copy,hashlib,json,random,struct,subprocess
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/attribute-edit');out=root/'cases';out.mkdir(exist_ok=True)
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def word(n):return struct.pack('<I',n)
def raw(v):return word(len(v))+v
def string(v):return raw(v.encode())
def optional(v):return word(0) if v is None else word(1)+string(v)
def attr(ordinal,ns,key,old,new,prefix=None,element='t'):
    return dict(ordinal=ordinal,element=['urn:a',element],attribute=[ns,key],old=old,new=new,prefix=prefix)
def wire(source,edits,stop=0,max_bytes=32*1024*1024,max_edits=10000,max_edit_bytes=32*1024*1024,max_attrs=256,max_attr_bytes=1024*1024):
    b=b''.join(map(word,[stop,max_bytes,max_edits,max_edit_bytes,max_attrs,max_attr_bytes]))+raw(source)+word(len(edits))
    for e in edits:b+=word(e['ordinal'])+b''.join(string(v) for v in e['element']+e['attribute'])+optional(e['old'])+optional(e['new'])+optional(e['prefix'])
    return b
def esc(s,q):
    return s.replace('&','&amp;').replace('<','&lt;').replace('\r','&#xD;').replace('\n','&#xA;').replace('\t','&#x9;').replace(q,'&apos;' if q=="'" else '&quot;')
def encode(s,enc,bom):
    prefix={'utf-8':b'\xef\xbb\xbf','utf-16le':b'\xff\xfe','utf-16be':b'\xfe\xff'}[enc] if bom else b''
    return prefix+s.encode(enc)
cases=[];r=random.Random(628)
alphabet=list('Az09 &<>\'"\r\n\té中🚀')+['e\u0301','\u202e','\u200d']
for i in range(64):
    old=''.join(r.choice(alphabet) for _ in range(r.randrange(1,40)))
    new=''.join(r.choice(alphabet) for _ in range(r.randrange(0,50)))
    quoted="'" if i%2==0 else '"'
    key='a' if i%4 else '名称';prefix='p' if i%3 else '前缀'
    start=f'<t {key} \t= {quoted}{esc(old,quoted)}{quoted} gone=\'drop\' {prefix}:keep = "opaque&amp;>" '
    end='/>' if i%2 else '><![CDATA[<t/>]]><!--keep中--><?future data?><future xmlns="urn:future"/></t>'
    changed=f'<t {key} \t= {quoted}{esc(new,quoted)}{quoted}  {prefix}:keep = "opaque&amp;>"  {prefix}:new="{esc(new,chr(34))}"'+end
    edits=[attr(1,'',key,old,new),attr(1,'','gone','drop',None),attr(1,'urn:p','new',None,new,prefix+':new')]
    if i%2:edits.reverse()
    for enc in ['utf-8','utf-16le','utf-16be']:
        for bom in [False,True]:
            declared='UTF-8' if enc=='utf-8' else 'UTF-16'
            head=f'<?xml version="1.0" encoding="{declared}"?><r xmlns="urn:a" xmlns:{prefix}="urn:p"><!--🚀-->'
            source=encode(head+start+end+'</r>',enc,bom);expected=encode(head+changed+'</r>',enc,bom)
            cases.append(dict(name=f'lexical-{i}-{enc}-{bom}',source=source,edits=edits,expected=expected,status=0))
source=b"<r xmlns='urn:a' xmlns:p='urn:p'><t a='old'/></r>";valid=[attr(1,'','a','old','new')]
cases.append(dict(name='noop',source=source,edits=[attr(1,'','a','old','old')],expected=source,status=0))
cases.append(dict(name='empty-edits',source=source,edits=[],expected=source,status=0))
for name,e in [
 ('wrong-old',[attr(1,'','a','stale','new')]),('absent-not-empty',[attr(1,'','a',None,'new')]),
 ('empty-not-absent',[attr(1,'','missing','',None)]),('missing-element',[attr(19,'','a',None,None)]),
 ('duplicate',valid+valid),('wrong-element',[attr(1,'','a','old','new',element='wrong')]),
 ('namespace-declaration',[attr(1,'','xmlns',None,'urn:new')]),
 ('namespace-uri',[attr(1,'http://www.w3.org/2000/xmlns/','p',None,'urn:new')]),
 ('wrong-prefix',[attr(1,'urn:other','new',None,'v','p:new')]),
 ('nul',[attr(1,'','a','old','\0')])]:
    cases.append(dict(name=name,source=source,edits=e,status=3))
for name,limits in [('edit-count',dict(max_edits=0)),('edit-bytes',dict(max_edit_bytes=1)),
    ('input-bytes',dict(max_bytes=1)),('attribute-count',dict(max_attrs=1)),('attribute-bytes',dict(max_attr_bytes=1))]:
    cases.append(dict(name=name,source=source,edits=valid,status=2,limits=limits))
for i,b in enumerate([b"<r>",b"<!DOCTYPE r [<!ENTITY a 'x'>]><r/>",b"<r x='<'/>",b"<r xmlns:p='urn:p' xmlns:q='urn:p' p:a='1' q:a='2'/>"]):
    cases.append(dict(name=f'malformed-{i}',source=b,edits=[],status=4))
for stop in range(1,12):cases.append(dict(name=f'cancel-{stop}',source=source,edits=valid,status=1,limits=dict(stop=stop)))
inputs=[]
for c in cases:inputs.append(wire(c['source'],c['edits'],**c.get('limits',{})))
(root/'probe-input.bin').write_bytes(word(len(inputs))+b''.join(raw(b) for b in inputs))
n=subprocess.run([str(root/'native-probe')],input=(root/'probe-input.bin').read_bytes(),capture_output=True)
assert n.returncode==0 and not n.stderr,(n.returncode,n.stderr)
(root/'probe-native.bin').write_bytes(n.stdout)
parser=X.XMLParser(resolve_entities=False,no_network=True,remove_blank_text=False)
records=[];at=0
for i,c in enumerate(cases):
    size=struct.unpack_from('<I',n.stdout,at)[0];at+=4;b=n.stdout[at:at+size];at+=size
    status,checks=struct.unpack_from('<II',b);assert status==c['status'],(c['name'],status,b[8:])
    source=out/f'{i}.xml';source.write_bytes(c['source'])
    record=dict(name=c['name'],source=entry(source),edits=c['edits'],status=status,cancellationChecks=checks)
    if status==0:
        assert b[8:]==c['expected'],c['name']
        before=X.fromstring(c['source'],parser);after=X.fromstring(b[8:],parser)
        elements=list(before.iter());elements=[e for e in elements if isinstance(e.tag,str)]
        for e in c['edits']:
            target=elements[e['ordinal']];ns,key=e['attribute'];key='{'+ns+'}'+key if ns else key
            assert target.get(key)==e['old']
            if e['new'] is None:target.attrib.pop(key,None)
            else:target.set(key,e['new'])
        assert X.tostring(before,method='c14n')==X.tostring(after,method='c14n'),c['name']
        output=out/f'{i}.edited.xml';output.write_bytes(b[8:]);record['output']=entry(output)
    records.append(record)
assert at==len(n.stdout)
w=subprocess.run(['node','tools/verification/attribute-edit-parity.mjs'],capture_output=True,text=True)
(root/'probe-wasm.log').write_text(w.stdout+w.stderr);assert w.returncode==0,w.stderr
report=dict(format='musteroffice.attribute-edit-reference/1',cases=records,pairedCalls=len(records),
    independentParser='lxml/libxml2',libxmlVersion=X.LIBXML_VERSION,successes=sum(c['status']==0 for c in records),
    native=entry(root/'native-probe'),wasm=entry(root/'probe.wasm'),input=entry(root/'probe-input.bin'),nativeOutput=entry(root/'probe-native.bin'),wasmOutput=entry(root/'probe-wasm.bin'))
(root/'reference.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(dict(cases=len(records),successes=report['successes'])))
