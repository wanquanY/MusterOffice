"""Build explicit queries from an independent parser, plus original malformed font probes."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.ttLib.tables.DefaultTable import DefaultTable

p=argparse.ArgumentParser();p.add_argument('directory',type=Path);p.add_argument('--corpus',type=Path,default=Path('.codex-work/font-corpus'));a=p.parse_args();a.directory.mkdir(parents=True,exist_ok=True)
cases=[]
def add(name,path,face=0,error=None):
    data=path.read_bytes();q={'expectedSha256':hashlib.sha256(data).hexdigest(),'faceIndex':face,'characters':[]}
    if not error:
        font=TTFont(path,fontNumber=face,recalcTimestamp=False)
        cmap={0x10000:'A',0x10001:'smile'} if name=='format-10' else (font.getBestCmap() or {})
        chars=set(cmap)|{0,32,65,0x301,0x3b1,0x627,0x915,0x4e2d,0x1f600,0x10ffff}
        q['characters']=[{'codepoint':cp,'variationSelector':None} for cp in sorted(chars)]
        for t in font['cmap'].tables:
            if t.format==14:
                for vs,values in t.uvsDict.items():
                    q['characters'] += [{'codepoint':cp,'variationSelector':vs} for cp,_ in values]
        q['characters'] += [{'codepoint':65,'variationSelector':vs} for vs in [0x180f,0xfe00,0xfe01,0xfe0f,0xe0100]]
    cases.append({'name':name,'path':str(path),'sha256':q['expectedSha256'],'request':q,'error':error})
for family in json.loads(Path('fixtures/fonts/upstream.json').read_text()):
    if family['name'].endswith('.ttf'):
        path=a.corpus/family['family']/family['name'];assert hashlib.sha256(path.read_bytes()).hexdigest()==family['sha256'];add(family['family'],path)
for file,faces in [('owned.ttf',1),('owned.otf',1),('owned.ttc',2)]:
    for face in range(faces):add(file.replace('.','-')+f'-{face}',Path('fixtures/fonts')/file,face)
for form,platform,encoding in [(0,0,0),(4,3,1),(6,0,3),(13,0,6)]:
    font=TTFont('fixtures/fonts/owned.ttf',recalcTimestamp=False)
    table=CmapSubtable.newSubtable(form);table.platformID=platform;table.platEncID=encoding;table.language=0
    table.cmap={32:'space',65:'A'};font['cmap'].tables=[table]
    path=a.directory/f'format-{form}.ttf';font.save(path);add(f'format-{form}',path)
# Format 10 is not parsed by FontTools 4.61.1; construct its standard header/array
# explicitly and verify it with a separate binary observer in font-independent.py.
font=TTFont('fixtures/fonts/owned.ttf',recalcTimestamp=False)
table=DefaultTable('cmap');table.data=struct.pack('>HHHHI',0,1,0,4,12)+struct.pack('>HHIIIIHH',10,0,24,0,0x10000,2,2,5)
font['cmap']=table;path=a.directory/'format-10.ttf';font.save(path);add('format-10',path)
# Malformed binary metadata has valid outer table/global checksums, so rejection
# must come from semantics, not a checksum failure that masks the defect.
def u16(b,p):return struct.unpack_from('>H',b,p)[0]
def u32(b,p):return struct.unpack_from('>I',b,p)[0]
def w16(b,p,v):struct.pack_into('>H',b,p,v)
def w32(b,p,v):struct.pack_into('>I',b,p,v)
def directory(b):return {bytes(b[p:p+4]):(p,u32(b,p+8),u32(b,p+12)) for p in range(12,12+u16(b,4)*16,16)}
def checksum(data):
    data=bytes(data)+b'\0'*((-len(data))%4)
    return sum(v[0] for v in struct.iter_unpack('>I',data))&0xffffffff
base=Path('fixtures/fonts/owned.ttf').read_bytes()
def mutated(name,fn,repair=True,error='FONT_INVALID'):
    b=bytearray(base);d=directory(b);fn(b,d)
    if repair:
        head=d[b'head'][1];w32(b,head+8,0)
        for p,start,length in d.values():w32(b,p+4,checksum(b[start:start+length]))
        w32(b,head+8,(0xb1b0afba-checksum(b))&0xffffffff)
    path=a.directory/(name+'.ttf');path.write_bytes(b);add(name,path,error=error)
def selected(b,d,form):
    p=d[b'cmap'][1]
    return next(p+u32(b,r+4) for r in range(p+4,p+4+u16(b,p+2)*8,8) if u16(b,p+u32(b,r+4))==form)
mutated('bad-directory-length',lambda b,d:w32(b,24,0xffffffff),False)
mutated('bad-cmap-offset',lambda b,d:w32(b,d[b'cmap'][1]+8,0xffffffff))
mutated('bad-cmap-glyph',lambda b,d:w32(b,selected(b,d,12)+24,65536))
mutated('bad-cmap-scalar',lambda b,d:w32(b,selected(b,d,12)+20,0x110000))
mutated('bad-cmap-uvs',lambda b,d:w32(b,selected(b,d,14)+17,1))
mutated('bad-cmap-length',lambda b,d:w32(b,selected(b,d,12)+4,0xffffffff))
mutated('bad-axis-range',lambda b,d:w32(b,d[b'fvar'][1]+u16(b,d[b'fvar'][1]+4)+4,1000<<16))
mutated('bad-head-em',lambda b,d:w16(b,d[b'head'][1]+18,0))
mutated('bad-name-storage',lambda b,d:w16(b,d[b'name'][1]+4,0xffff))
mutated('bad-name-language',lambda b,d:w16(b,d[b'name'][1]+10,0x8000))
mutated('bad-instance-size',lambda b,d:w16(b,d[b'fvar'][1]+14,1))
mutated('bad-table-checksum',lambda b,d:b.__setitem__(-1,b[-1]^1),False)
mutated('bad-file-checksum',lambda b,d:w32(b,d[b'head'][1]+8,0),False)
# Unsupported highest-priority cmap must be reported, not silently skipped.
mutated('unsupported-cmap',lambda b,d:w16(b,selected(b,d,12),8),error='UNSUPPORTED')
for name,data,error in [('truncated',base[:100],'FONT_INVALID'),('woff',b'wOFF'+bytes(64),'UNSUPPORTED')]:
    path=a.directory/(name+'.bin');path.write_bytes(data);add(name,path,error=error)
(a.directory/'manifest.json').write_text(json.dumps({'format':'musteroffice.font-fixtures/1','cases':cases},indent=2)+'\n')
print(json.dumps({'cases':len(cases),'queries':sum(len(c['request']['characters']) for c in cases)}))
