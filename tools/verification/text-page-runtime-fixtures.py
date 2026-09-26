"""Bind the previous independent pixel oracle and generate native XML failures."""
import hashlib
import io
import json
from pathlib import Path
import re
import zipfile

ROOT = Path('.codex-work/text-page-runtime')
ROOT.mkdir(parents=True, exist_ok=True)
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-source-text-page-library-verification.json')
def entry(path):
    path = Path(path); data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())
assert entry(PREVIOUS)['sha256'] == '5f39994c5f0e0c7fe53fc7e8114d2d1ec650f9a1732ec07c797bb9208364c0b5'
old = json.loads(PREVIOUS.read_text())
positive = old['independentReference']['cases']
for case in positive:
    for key in ['source', 'result', 'pixels']:
        assert entry(case[key]['path']) == case[key]
source = Path(positive[0]['source']['path']).read_bytes()
def change(name, fn, detail):
    output = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(source)) as zin, zipfile.ZipFile(output, 'w') as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename == 'ppt/slides/slide1.xml':
                old_xml = data.decode(); new_xml = fn(old_xml)
                assert new_xml != old_xml, name
                data = new_xml.encode()
            zout.writestr(item, data)
    path = ROOT / (name + '.pptx'); path.write_bytes(output.getvalue())
    return dict(name=name, source=entry(path), detail=detail)
def later_underline(s):
    at = s.rindex('<a:rPr>')
    return s[:at] + s[at:].replace('<a:rPr>', '<a:rPr u="sng">', 1)
def no_text_fill(s):
    start, end = s.index('<p:txBody>'), s.index('</p:txBody>')
    return s[:start] + re.sub(r'<a:solidFill>.*?</a:solidFill>', '', s[start:end]) + s[end:]
def cluster(s):
    at = s.index('<a:t>A</a:t>') + len('<a:t>A</a:t>')
    return s[:at] + s[at:].replace('<a:t>A</a:t>', '<a:t>&#x301;</a:t>', 1)
negative = [
    change('later-underline', later_underline, 'paintProperty'),
    change('autofit', lambda s: s.replace('<a:noAutofit/>', '<a:spAutoFit/>', 1), 'frame'),
    change('missing-paint', no_text_fill, 'missingPaint'),
    change('mixed-cluster-paint', cluster, 'glyphPaintConflict'),
]
(ROOT/'fixtures.json').write_text(json.dumps(dict(previousEvidence=entry(PREVIOUS), positive=positive, negative=negative), indent=2)+'\n')
print(json.dumps(dict(positive=len(positive), negative=len(negative))))
