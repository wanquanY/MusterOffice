"""Official UAX #29 cases and independent range/dense-property verification."""
import array,hashlib,json,re,struct
from pathlib import Path
root=Path('.codex-work/unicode');out=root/'verification';out.mkdir(exist_ok=True)
lock=json.loads(Path('crates/mo-unicode/data/manifest.json').read_text())
for f in lock['inputs']:
    data=(root/f['name']).read_bytes();assert hashlib.sha256(data).hexdigest()==f['sha256'] and len(data)==f['byteLength']
tests=[]
for line_number,line in enumerate((root/'GraphemeBreakTest-18.0.0.txt').read_text().splitlines(),1):
    body=line.partition('#')[0].strip()
    if not body:continue
    text=[];boundaries=[];utf8=utf16=0
    for token in body.split():
        if token=='÷':boundaries.append({'scalarOffset':len(text),'utf8Offset':utf8,'utf16Offset':utf16})
        elif token!='×':
            cp=int(token,16);c=chr(cp);text.append(c);utf8+=len(c.encode());utf16+=1+(cp>65535)
    tests.append({'line':line_number,'text':''.join(text),'boundaries':boundaries})
# The independent oracle expands textual facts densely, then checks every point
# against the generated binary. Runtime queries also exercise range boundaries.
gcb_names=['Other','CR','LF','Control','Extend','ZWJ','Regional_Indicator','Prepend','SpacingMark','L','V','T','LV','LVT']
gcb_wire=['other','cr','lf','control','extend','zwj','regionalIndicator','prepend','spacingMark','l','v','t','lv','lvt']
incb_names=['None','Consonant','Extend','Linker'];incb_wire=['none','consonant','extend','linker']
dense=array.array('H',[0])*0x110000
pattern=re.compile(r'^([0-9A-F]+)(?:\.\.([0-9A-F]+))?\s*;\s*([^#]+)',re.M)
probes={0,0x10ffff,0xd7ff,0xe000}
for filename in ['GraphemeBreakProperty','DerivedCoreProperties','emoji-data','PropList']:
    source=(root/f'{filename}-18.0.0.txt').read_text()
    for match in pattern.finditer(source):
        start=int(match[1],16);end=int(match[2] or match[1],16);fields=[v.strip() for v in match[3].strip().split(';')]
        bits=0
        if filename=='GraphemeBreakProperty':bits=gcb_names.index(fields[0])
        elif fields[0]=='InCB':bits=incb_names.index(fields[1])<<4
        elif fields[0]=='Default_Ignorable_Code_Point':bits=128
        elif fields[0]=='Extended_Pictographic':bits=64
        elif fields[0]=='Variation_Selector':bits=256
        else:continue
        for cp in range(start,end+1):dense[cp]|=bits
        probes.update([start-1,start,end,(start+end)//2,end+1])
binary=Path('crates/mo-unicode/data/properties.bin').read_bytes();assert binary[:8]==b'MOUCD018'
observed=array.array('H',[0])*0x110000
for start,end,flags in struct.iter_unpack('<IIH',binary[12:]):
    for cp in range(start,end+1):observed[cp]=flags
assert observed==dense
def props(cp):
    f=dense[cp]
    return {'codepoint':cp,'properties':{'graphemeBreak':gcb_wire[f&15],'indicConjunct':incb_wire[(f>>4)&3],'extendedPictographic':bool(f&64),'defaultIgnorable':bool(f&128),'variationSelector':bool(f&256)}}
probes=sorted(c for c in probes if 0<=c<=0x10ffff and not 0xd800<=c<=0xdfff)
case={'format':'musteroffice.unicode-fixtures/1','unicodeVersion':'18.0.0','officialGraphemeCases':tests,'propertyProbes':[props(c) for c in probes],
      'denseCodepointsCompared':0x110000,'denseLittleEndianSha256':hashlib.sha256(struct.pack('<'+'H'*len(dense),*dense)).hexdigest()}
(out/'fixtures.json').write_text(json.dumps(case,ensure_ascii=False,separators=(',',':'))+'\n')
print(json.dumps({'officialGraphemeCases':len(tests),'propertyProbes':len(probes),'denseCodepointsCompared':0x110000}))
