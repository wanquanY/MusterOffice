"""Independent XML/MCE body cascade and part-schema checks for owned probes."""
import hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import project,A,P
ROOT=Path('.codex-work/text-style');N={'a':A,'p':P};XSD=Path('.codex-work/ecma376/xsd')
sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(Path(p).read_text())
archive=XSD.parent/'OfficeOpenXML-XMLSchema-Transitional.zip'
assert sha(archive.read_bytes())=='d34187520749998af306faf1b730e568b0ca6d88ad24638a407c0a9bb4ca04fc'
with zipfile.ZipFile(archive) as z:
 for name in z.namelist():
  if name.endswith('.xsd'):assert (XSD/name).read_bytes()==z.read(name),name
pml=E.XMLSchema(E.parse(str(XSD/'pml.xsd'),E.XMLParser(resolve_entities=False,no_network=True)))
dml=E.XMLSchema(E.parse(str(XSD/'dml-main.xsd'),E.XMLParser(resolve_entities=False,no_network=True)))
attrs={'rot':('rotation',int,0),'spcFirstLastPara':('paragraphSpacing',lambda v:v in ['true','1'],False),'vertOverflow':('verticalOverflow',str,'overflow'),'horzOverflow':('horizontalOverflow',str,'overflow'),'vert':('vertical',str,'horz'),'wrap':('wrap',str,'square'),'lIns':('leftInset',str,'91440'),'tIns':('topInset',str,'45720'),'rIns':('rightInset',str,'91440'),'bIns':('bottomInset',str,'45720'),'numCol':('columns',int,1),'spcCol':('columnSpacing',str,'0'),'rtlCol':('rightToLeftColumns',lambda v:v in ['true','1'],False),'fromWordArt':('fromWordArt',lambda v:v in ['true','1'],False),'anchor':('anchor',str,'t'),'anchorCtr':('centerAnchor',lambda v:v in ['true','1'],False),'forceAA':('forceAntialiasing',lambda v:v in ['true','1'],False),'upright':('upright',lambda v:v in ['true','1'],False),'compatLnSpc':('compatibleLineSpacing',lambda v:v in ['true','1'],False)}
counts={'bodies':0,'propertyValues':0,'propertyOrigins':0,'autofitChoices':0,'validParts':0,'invalidParts':0,'unchangedCompressedParts':0};cases=[];invalid=[]
def physical(z,part):
 raw=z.read(part[1:]);root,_,ordinals=project(raw,True);return root,ordinals

def body_of(root,ident):
 shapes=root.xpath(f'.//p:sp[p:nvSpPr/p:cNvPr/@id="{ident}"]',namespaces=N);assert len(shapes)<=1
 return None if not shapes else shapes[0].find('p:txBody/a:bodyPr',N)
def raw_compressed(z,name):
 import struct
 entry=z.getinfo(name);z.fp.seek(entry.header_offset);h=z.fp.read(30);names,extra=struct.unpack_from('<HH',h,26);z.fp.seek(names+extra,1);return z.fp.read(entry.compress_size)
for c in read(ROOT/'manifest.json')['cases']:
 src=Path(c['sourcePath']).read_bytes();assert sha(src)==c['sourceSha256'];r=read(ROOT/(c['name']+'.response.json'))
 with zipfile.ZipFile(c['sourcePath']) as z:
  parts={p:physical(z,c[p]) for p in ['slide','layout','master','theme']}
  for p in parts:
   root,_=parts[p];valid=(dml if p=='theme' else pml).validate(root)
   counts['validParts' if valid else 'invalidParts']+=1
   if not valid:invalid.append({'name':c['name'],'part':c[p],'reason':str((dml if p=='theme' else pml).error_log.last_error)})
  if c['status']=='error':assert r['status']=='error';continue
  out=r['styles']['objects'][0]['outcome'];assert out['status']==c['status']
  if c['status']=='resolved':
   chain=[]
   def add_body(p):
    root,ordinals=parts[p];body=body_of(root,c['object'])
    if body is not None:chain.append((body,ordinals,{'kind':'object','object':{'part':c[p],'nativeId':c['object']}}))
   add_body('slide');add_body('layout')
   theme,ordinals=parts['theme']
   for kind in ['txDef','lnDef','spDef']:
    body=theme.find(f'a:objectDefaults/a:{kind}/a:bodyPr',N)
    if body is not None:chain.append((body,ordinals,{'kind':'theme','part':c['theme'],'defaultKind':kind}))
   add_body('master')
   body=out['body'];counts['bodies']+=1
   for native,(field,convert,default) in attrs.items():
    found=next(((n,o,origin) for n,o,origin in chain if native in n.attrib),None)
    if found:
     n,o,origin=found;expected=convert(n.get(native));origin={**origin,'sourceOrdinal':o[n]}
    else:expected=default;origin={'kind':'profileDefault'}
    assert body['attributes'][field]==expected,(c['name'],field,body['attributes'][field],expected)
    assert body['origins'][field]==origin,(c['name'],field,body['origins'][field],origin)
    counts['propertyValues']+=1;counts['propertyOrigins']+=1
   fit=None
   for n,o,origin in chain:
    choices=[child for child in n if E.QName(child).localname in ['noAutofit','normAutofit','spAutoFit']]
    if not choices:continue
    n=choices[0];kind=E.QName(n).localname;fit={'kind':{'noAutofit':'none','normAutofit':'normal','spAutoFit':'shape'}[kind],'declaredBy':{**origin,'sourceOrdinal':o[n]}}
    if kind=='normAutofit':fit.update(fontScale=n.get('fontScale','100000'),lineSpacingReduction=n.get('lnSpcReduction','0'),fontScaleDefaulted=n.get('fontScale') is None,lineSpacingReductionDefaulted=n.get('lnSpcReduction') is None)
    break
   fit=fit or {'kind':'none','declaredBy':{'kind':'profileDefault'}};assert body['autofit']==fit,(c['name'],body['autofit'],fit);counts['autofitChoices']+=1
  with zipfile.ZipFile(ROOT/(c['name']+'.edited.pptx')) as edited:
   assert z.namelist()==edited.namelist()
   for name in z.namelist():
    if name==c['slide'][1:]:
     old=z.read(name);new=edited.read(name);assert new==old.replace(b'<a:t>Native text</a:t>', '<a:t>Native body preserved 中ع</a:t>'.encode(),1)
    else:assert raw_compressed(z,name)==raw_compressed(edited,name);counts['unchangedCompressedParts']+=1
 cases.append({'name':c['name'],'sourceSha256':c['sourceSha256'],'responsePath':str(ROOT/(c['name']+'.response.json')),'responseSha256':sha((ROOT/(c['name']+'.response.json')).read_bytes())})
report={'format':'musteroffice.text-body-reference/1','scope':'Independent owned XML property cascade and physical provenance. Draft precedence, not Office/WPS acceptance, font binding or text layout. Invalid probes listed separately.','counts':counts,'invalidParts':invalid,'cases':cases}
(ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(counts))
