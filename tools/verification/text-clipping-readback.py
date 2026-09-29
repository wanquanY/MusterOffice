"""Validate native clipping corpus XML and public page/playback contracts."""
import argparse,hashlib,json,pathlib,zipfile
from lxml import etree
from jsonschema import Draft202012Validator
parser=argparse.ArgumentParser(description=__doc__)
for key in ['corpus-dir','parity-report','schema-dir','contracts-dir','output']:
 parser.add_argument('--'+key,type=pathlib.Path,required=True)
args=parser.parse_args()
assert not args.output.exists(), 'do not overwrite evidence'
report=json.loads(args.parity_report.read_text())
contracts=args.contracts_dir
cache={}
def check(name,record):
 if name not in cache:cache[name]=Draft202012Validator(json.loads((contracts/(name+'.schema.json')).read_text()))
 path=pathlib.Path(record['path']);data=path.read_bytes();assert hashlib.sha256(data).hexdigest()==record['sha256'];cache[name].validate(json.loads(data))
count=0
for row in report['staticCases']:
 mode=row['mode'];stem='pptx-text-page' if mode=='text' else 'pptx-resource-page'
 check(stem+'-request',row['request']);check(stem+'-raster-response',row['response']);count+=2
prepared=set()
for row in report['playbackCases']:
 check('pptx-playback-session-request',row['request']);check('pptx-playback-session-response',row['response']);count+=2
 if row['prepared']['path'] not in prepared:
  check('pptx-playback-session-response',row['prepared']);prepared.add(row['prepared']['path']);count+=1
xsd=etree.XMLSchema(etree.parse(str(args.schema_dir/'pml.xsd')))
parser=etree.XMLParser(resolve_entities=False,no_network=True)
files=[];xml_count=0
for p in sorted(args.corpus_dir.rglob('*.pptx')):
 with zipfile.ZipFile(p) as package:
  for name in package.namelist():
   if not name.endswith('.xml'):continue
   doc=etree.fromstring(package.read(name),parser)
   if doc.tag.startswith('{http://schemas.openxmlformats.org/presentationml/2006/main}'):
    xsd.assertValid(doc);xml_count+=1
 files.append(dict(name=str(p.relative_to(args.corpus_dir)),sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
assert len(files)==47 and len(prepared)==15 and count==323
out=dict(status='passed',jsonSchemaValidations=count,pptxFiles=len(files),xsdParts=xml_count,files=files,contracts={n:hashlib.sha256((contracts/(n+'.schema.json')).read_bytes()).hexdigest() for n in cache})
args.output.write_text(json.dumps(out,indent=2)+'\n');print({k:v for k,v in out.items() if k not in ['files','contracts']})
