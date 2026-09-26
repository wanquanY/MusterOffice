"""Owned native text/resource and physical-run diagnostic scenarios."""
import hashlib
import io
import json
from pathlib import Path
import zipfile

ROOT=Path('.codex-work/text-resource-diagnostics')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-text-page-runtime-verification.json')
def entry(path):
    p=Path(path); b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(PREVIOUS)['sha256']=='4ce18b01e729d4a28b5d53f6e165c419e868427637ac71b35791d4798e787a0e'
old=json.loads(PREVIOUS.read_text())
base=old['parityEvidence']['cases'][0]
source=Path(base['source']['path']).read_bytes(); assert entry(base['source']['path'])==base['source']
def modified(name, fn):
    output=io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(source)) as zin, zipfile.ZipFile(output,'w') as zout:
        for item in zin.infolist():
            b=zin.read(item.filename)
            if item.filename=='ppt/slides/slide1.xml':
                previous=b.decode();updated=fn(previous);assert updated!=previous;b=updated.encode()
            zout.writestr(item,b)
    path=ROOT/(name+'.pptx');path.write_bytes(output.getvalue());return entry(path)
bold=modified('bold-run',lambda s:s.replace('<a:rPr>','<a:rPr b="1">',1))
insertion=modified('italic-insertion',lambda s:s.replace('</a:p>','<a:endParaRPr i="1"/></a:p>',1))
paragraph=modified('later-paragraph',lambda s:s.replace('</a:p>','</a:p><a:p><a:r><a:rPr b="1"><a:solidFill><a:srgbClr val="102030"/></a:solidFill></a:rPr><a:t>A</a:t></a:r></a:p>',1))
def second_run(s,transform):
    at=s.index('<a:r>',s.index('<a:r>')+len('<a:r>'))
    return s[:at]+transform(s[at:])
color=modified('system-color',lambda s:second_run(s,lambda t:t.replace('<a:srgbClr val="2070C0"/>','<a:sysClr val="window"/>',1)))
paint=modified('later-paragraph-underline',lambda s:s.replace('</a:p>','</a:p><a:p><a:r><a:rPr u="sng"><a:solidFill><a:srgbClr val="102030"/></a:solidFill></a:rPr><a:t>A</a:t></a:r></a:p>',1))
cases=[
    dict(name='missing-family',source=base['source'],change='missingFamily',reason='unmappedTypeface',style='regular',paragraph=0,runs=[0,1,None]),
    dict(name='missing-regular',source=base['source'],change='missingRegular',reason='missingStyle',style='regular',paragraph=0,runs=[0,1,None]),
    dict(name='bold-run',source=bold,reason='missingStyle',style='bold',paragraph=0,runs=[0]),
    dict(name='italic-insertion',source=insertion,reason='missingStyle',style='italic',paragraph=0,runs=[None]),
    dict(name='later-paragraph',source=paragraph,reason='missingStyle',style='bold',paragraph=1,runs=[0]),
    dict(name='system-color',source=color,paint='color',paragraph=0,run=1,recoverColor=True),
    dict(name='later-paragraph-underline',source=paint,paint='paintProperty',paragraph=1,run=0),
]
(ROOT/'fixtures.json').write_text(json.dumps(dict(previousEvidence=entry(PREVIOUS),base=base,cases=cases),indent=2)+'\n')
print(json.dumps(dict(cases=len(cases))))
