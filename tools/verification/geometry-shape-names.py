"""Generate the native shape-name vocabulary from the pinned official XSD.

Only schema enumeration facts are generated, not preset geometry templates.
"""
import argparse,hashlib,json
from pathlib import Path
from xml.etree import ElementTree as E
p=argparse.ArgumentParser();p.add_argument('mode',choices=['write','check']);args=p.parse_args()
xsd=Path('.codex-work/ecma376/xsd/dml-main.xsd');raw=xsd.read_bytes()
assert hashlib.sha256(raw).hexdigest()=='6978ba7e889070b0c3cb5b546b23e5a6c3516134afc53b87a21f482ca33f3858'
ns={'x':'http://www.w3.org/2001/XMLSchema'}
names=sorted(n.get('value') for n in E.fromstring(raw).findall("x:simpleType[@name='ST_ShapeType']/x:restriction/x:enumeration",ns));assert len(names)==len(set(names))
content='// Generated schema enumeration facts by geometry-shape-names.py.\n// ECMA-376 Part 4 Transitional dml-main.xsd ST_ShapeType.\npub const NAMES: &[&str] = &[\n'+''.join('    '+json.dumps(n)+',\n' for n in names)+'];\n'
path=Path('crates/mo-pptx/src/source/geometry/names.rs')
if args.mode=='write':path.write_text(content)
else:assert path.read_text()==content
print(json.dumps({'shapeNames':len(names),'xsdSha256':hashlib.sha256(raw).hexdigest(),'generatedSha256':hashlib.sha256(content.encode()).hexdigest()}))
