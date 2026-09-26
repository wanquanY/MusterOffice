"""Attribute the actual WASM byte delta and rerun existing public API cases."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

root = Path('.codex-work/embedded-export')
initial = json.loads((root/'wasm.json').read_text())

def entry(path):
    path=Path(path); raw=path.read_bytes()
    return dict(path=str(path),byteLength=len(raw),sha256=hashlib.sha256(raw).hexdigest())
def sha(raw): return hashlib.sha256(raw).hexdigest()
def leb(raw, at, signed=False):
    value=0; shift=0
    while True:
        byte=raw[at]; at+=1; value |= (byte & 127)<<shift; shift+=7
        assert shift<=35
        if byte<128: break
    if signed and byte&64: value-=1<<shift
    return value,at

def sections(raw):
    assert raw[:8] == b'\0asm\1\0\0\0'
    result=[]; at=8
    while at<len(raw):
        kind=raw[at]; length,start=leb(raw,at+1); at=start+length
        assert at<=len(raw)
        name=''
        if kind==0:
            size,i=leb(raw,start); name=raw[i:i+size].decode()
        result.append(dict(kind=kind,name=name,start=start,byteLength=length,sha256=sha(raw[start:at])))
    return result

old=Path(initial['previousModule']['path']).read_bytes()
new=Path(initial['module']['path']).read_bytes()
assert len(old)==len(new) and initial['glueUnchanged']
before=sections(old); after=sections(new)
different=[]
for a,b in zip(before,after,strict=True):
    assert all(a[k]==b[k] for k in ['kind','name','start','byteLength'])
    if a['sha256']!=b['sha256']: different.append(b['kind'])
assert different == [11], different
data=next(s for s in after if s['kind']==11)
count,at=leb(new,data['start'])
segments=[]
for _ in range(count):
    flag,at=leb(new,at); assert flag==0
    assert new[at]==0x41; at+=1
    address,at=leb(new,at,True)
    assert new[at]==0x0b; at+=1
    length,at=leb(new,at)
    segments.append((address,at,length)); at+=length
assert at==data['start']+data['byteLength']
differences=[i for i,(a,b) in enumerate(zip(old,new)) if a!=b]
assert len(differences)==2
locations=[]
for offset in differences:
    pointer=int.from_bytes(new[offset-8:offset-4],'little')
    length=int.from_bytes(new[offset-4:offset],'little')
    owners=[(address,start) for address,start,size in segments if address<=pointer and pointer+length<=address+size]
    assert len(owners)==1
    address,start=owners[0]
    filename=new[start+pointer-address:start+pointer-address+length].decode()
    previous=int.from_bytes(old[offset:offset+4],'little')
    current=int.from_bytes(new[offset:offset+4],'little')
    column=int.from_bytes(new[offset+4:offset+8],'little')
    line=Path(filename).read_text().splitlines()[current-1]
    assert current==previous+5 and ('unreachable!' in line or '.expect(' in line)
    locations.append(dict(file=filename,oldLine=previous,newLine=current,column=column,byteOffset=offset,sourceLine=line))
assert {x['file'] for x in locations} == {
    'crates/mo-kernel-api/src/text.rs',
    'crates/mo-kernel-api/src/pptx_resource_page/diagnostic.rs',
}

cli=Path('target/debug/mo-cli')
copy=root/'frozen/mo-cli'
with copy.open('xb') as stream:stream.write(cli.read_bytes())
copy.chmod(0o700)
records=[]
def run(name,argv):
    log=root/f'{name}.log'
    with log.open('x') as stream:
        result=subprocess.run(argv,stdout=stream,stderr=subprocess.STDOUT)
    records.append(dict(name=name,argv=argv,exitCode=result.returncode,log=entry(log)))
    print(json.dumps(records[-1]),flush=True)
    assert result.returncode==0,log
run('reception', ['node','tools/verification/delivery-receive-parity.mjs',
    str(root/'reception'),str(copy),str(root/'wasm-node/mo_wasm.js')])
original=Path('tools/verification/delivery-receive-regression.py')
source=original.read_text()
assert source.count("root = Path('.codex-work/delivery-receive')")==1
driver=root/'regression-driver.py'
with driver.open('x') as stream:
    stream.write(source.replace("root = Path('.codex-work/delivery-receive')",f"root = Path('{root}')"))
run('regression', ['python3',str(driver)])
report=dict(format='musteroffice.embedded-export-wasm-analysis/1',
    module=initial['module'],glue=initial['glue'],previousModule=initial['previousModule'],
    previousGlue=initial['previousGlue'],moduleUnchanged=False,glueUnchanged=True,
    unchangedCodeImportsExportsAndAllOtherSections=True,
    oldSections=before,newSections=after,changedBytes=len(differences),panicSourceLocations=locations,
    checks=records,nativeCli=entry(copy),reception=entry(root/'reception/report.json'),
    regression=entry(root/'regression.json'),originalRegressionDriver=entry(original),
    scope='Only two panic source line numbers changed; executable code and every other section are byte-identical. Public reception plus file/edit cases were rerun; not new full playback or browser-owner acceptance.')
with (root/'wasm-analysis.json').open('x') as stream:
    json.dump(report,stream,indent=2);stream.write('\n')
print(json.dumps(dict(changedBytes=2,panicLocations=locations,pairedCalls=108)))

