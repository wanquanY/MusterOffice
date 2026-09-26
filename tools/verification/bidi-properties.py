"""Independent dense UCD reference and actual-API probe fixtures for bidi data."""
import argparse,hashlib,json,re,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/bidi'));a=p.parse_args()
manifest=json.loads(Path('crates/mo-unicode/data/bidi-manifest.json').read_text());source={}
for row in manifest['inputs']:
    data=(a.directory/row['name']).read_bytes();assert len(data)==row['byteLength'] and hashlib.sha256(data).hexdigest()==row['sha256'];source[row['name'].split('-')[0]]=data.decode()
names='L R AL EN ES ET AN CS NSM BN B S WS ON LRE LRO RLE RLO PDF LRI RLI FSI PDI'.split()
classes=bytearray(0x110000);mirrored=bytearray(0x110000)
aliases={'Left_To_Right':'L','Right_To_Left':'R','Arabic_Letter':'AL','European_Terminator':'ET'}
pattern=r'([0-9A-F]+)(?:\.\.([0-9A-F]+))?\s*;\s*(\w+)'
for raw in source['DerivedBidiClass'].splitlines():
    if '@missing:' in raw:
        m=re.search(pattern,raw);assert m;lo=int(m[1],16);hi=int(m[2] or m[1],16);v=names.index(aliases[m[3]]);classes[lo:hi+1]=bytes([v])*(hi-lo+1)
for raw in source['DerivedBidiClass'].splitlines():
    m=re.fullmatch(pattern,raw.partition('#')[0].strip())
    if m:
        lo=int(m[1],16);hi=int(m[2] or m[1],16);classes[lo:hi+1]=bytes([names.index(m[3])])*(hi-lo+1)
decomposition={};first=None
for raw in source['UnicodeData'].splitlines():
    f=raw.split(';');cp=int(f[0],16)
    if f[1].endswith(', First>'):first=(cp,f[9]);continue
    if f[1].endswith(', Last>'):
        assert first and first[1]==f[9]
        if f[9]=='Y':mirrored[first[0]:cp+1]=b'\1'*(cp-first[0]+1)
        first=None;continue
    if f[9]=='Y':mirrored[cp]=1
    if f[5] and not f[5].startswith('<'):decomposition[cp]=tuple(int(x,16) for x in f[5].split())
assert first is None
def normalized(cp):
    result=[cp]
    for _ in range(16):
        expanded=[item for c in result for item in decomposition.get(c,(c,))]
        if expanded==result:assert len(result)==1;return result[0]
        result=expanded
    raise AssertionError('canonical cycle')
brackets={};mirrors={}
for raw in source['BidiBrackets'].splitlines():
    f=[v.strip() for v in raw.partition('#')[0].split(';')]
    if not f[0]:continue
    cp,pair=int(f[0],16),int(f[1],16);is_open=f[2]=='o'
    brackets[cp]={'paired':pair,'normalizedOpening':normalized(cp if is_open else pair),'isOpen':is_open}
for raw in source['BidiMirroring'].splitlines():
    f=[v.strip() for v in raw.partition('#')[0].split(';')]
    if f[0]:mirrors[int(f[0],16)]=int(f[1],16)
data=Path('crates/mo-unicode/data/bidi.bin').read_bytes();assert data[:8]==b'MOBDI018';nr,nb,nm=struct.unpack_from('<III',data,8)
assert len(data)==20+9*nr+13*nb+8*nm
ranges=[struct.unpack_from('<IIB',data,20+i*9) for i in range(nr)]
probes={0,0x10ffff,0x221d,0x1db10,0x2e62,0x2e63};previous=-1
actual=bytearray(0x110000)
for lo,hi,bits in ranges:
    assert previous<lo<=hi<0x110000 and 0<bits<64 and (bits&31)<len(names);previous=hi
    actual[lo:hi+1]=bytes([bits])*(hi-lo+1)
    probes.update([lo,hi,(lo+hi)//2,lo-1,hi+1])
for cp in range(0x110000):assert actual[cp]==classes[cp]+32*mirrored[cp],hex(cp)
at=20+9*nr;actual_brackets={}
for i in range(nb):
    cp,pair,opening,is_open=struct.unpack_from('<IIIB',data,at+i*13);assert cp not in actual_brackets and is_open in (0,1)
    actual_brackets[cp]={'paired':pair,'normalizedOpening':opening,'isOpen':bool(is_open)}
assert actual_brackets==brackets
at+=nb*13;actual_mirrors=dict(struct.unpack_from('<II',data,at+i*8) for i in range(nm));assert len(actual_mirrors)==nm and actual_mirrors==mirrors
probes.update(brackets);probes.update(mirrors);probes.update(mirrors.values())
characters=sorted(cp for cp in probes if 0<=cp<=0x10ffff and not 0xd800<=cp<=0xdfff)
expected=[{'codepoint':cp,'properties':{'class':names[classes[cp]],'bracket':brackets.get(cp),'mirrored':bool(mirrored[cp]),'mirroringGlyph':mirrors.get(cp)}} for cp in characters]
result={'format':'musteroffice.bidi-property-fixtures/1','denseCodepointsCompared':0x110000,'ranges':nr,'brackets':nb,'mirroringMappings':nm,'mirroredCodepoints':sum(mirrored),'runtimeTableSha256':hashlib.sha256(data).hexdigest(),'characters':characters,'expected':expected}
(a.directory/'property-fixtures.json').write_text(json.dumps(result,ensure_ascii=False,separators=(',',':'))+'\n')
print(json.dumps({'denseCodepointsCompared':0x110000,'probes':len(characters),'ranges':nr,'brackets':nb,'mirroringMappings':nm,'mirroredCodepoints':sum(mirrored)}))
