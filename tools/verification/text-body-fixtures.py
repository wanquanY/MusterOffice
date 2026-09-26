"""Owned native text-body/default hierarchy probes; no external document assets."""
import hashlib,io,json,re,zipfile
from pathlib import Path
ROOT=Path('.codex-work/text-style');ROOT.mkdir(exist_ok=True)
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-source-text-verification.json')
old=json.loads(PREVIOUS.read_text());case=next(c for c in old['textReport']['cases'] if c['name']=='inspect-empty-properties')
sha=lambda b:hashlib.sha256(b).hexdigest()
source=Path(case['sourcePath']).read_bytes();assert sha(source)==case['sourceSha256']
r=Path(case['responsePath']).read_bytes();assert sha(r)==case['responseSha256'];index=json.loads(r)['index']
slide='/ppt/slides/slide1.xml';surface=index['surfaces'][slide];obj=surface['objects'][0]['nativeId'];layout=surface['links']['layout'];master=index['surfaces'][layout]['links']['master'];theme=surface['effectiveTheme']['base']['part']
with zipfile.ZipFile(io.BytesIO(source)) as z:base={n:z.read(n) for n in z.namelist()}
def body(text,properties):return re.sub(r'<p:txBody>.*?</p:txBody>',lambda _:f'<p:txBody>{properties}<a:lstStyle/><a:p><a:r><a:t>Native text</a:t></a:r></a:p></p:txBody>',text,count=1,flags=re.S)
def definition(kind,props,list_style=''):return f'<a:{kind}><a:spPr/>{props}<a:lstStyle>{list_style}</a:lstStyle></a:{kind}>'
cases=[]
def save(name,props='<a:bodyPr/>',defs='',parents=None,status='resolved'):
 parts=dict(base);s=body(parts[slide[1:]].decode(),props)
 if parents is not None:
  shape=re.search(r'<p:sp>.*?</p:sp>',s,re.S).group();shape=shape.replace('<p:nvPr/>','<p:nvPr><p:ph type="body" idx="0"/></p:nvPr>',1);s=re.sub(r'<p:sp>.*?</p:sp>',lambda _:shape,s,count=1,flags=re.S)
  for part,p in zip([layout,master],parents,strict=True):
   value=body(shape,p);parts[part[1:]]=parts[part[1:]].decode().replace('</p:spTree>',value+'</p:spTree>').encode()
 parts[slide[1:]]=s.encode()
 if defs:parts[theme[1:]]=parts[theme[1:]].decode().replace('</a:theme>',f'<a:objectDefaults>{defs}</a:objectDefaults></a:theme>').encode()
 out=io.BytesIO()
 with zipfile.ZipFile(out,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=6) as z:
  for n,b in parts.items():
   info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
 b=out.getvalue();path=ROOT/(name+'.pptx');path.write_bytes(b);cases.append({'name':name,'sourcePath':str(path),'sourceSha256':sha(b),'status':status,'object':obj,'slide':slide,'layout':layout,'master':master,'theme':theme})
save('profile-defaults')
attrs={'rot':'-21600001','spcFirstLastPara':'true','vertOverflow':'clip','horzOverflow':'clip','vert':'eaVert','wrap':'none','lIns':'-0.000000001pt','tIns':'0','rIns':'+00001','bIns':'1.23456789mm','numCol':'16','spcCol':'0','rtlCol':'true','fromWordArt':'true','anchor':'ctr','anchorCtr':'true','forceAA':'true','upright':'true','compatLnSpc':'true'}
for key,val in attrs.items():
 for origin in ['local','txDef','lnDef','spDef']:
  prop=f'<a:bodyPr {key}="{val}"/>'
  save(key+'-'+origin,prop if origin=='local' else '<a:bodyPr/>', '' if origin=='local' else definition(origin,prop))
for choice in ['noAutofit','normAutofit','spAutoFit']:
 for origin in ['local','txDef','master']:
  prop=f'<a:bodyPr><a:{choice}/></a:bodyPr>'
  save(choice+'-'+origin,prop if origin=='local' else '<a:bodyPr/>',definition(origin,prop) if origin=='txDef' else '',['<a:bodyPr/>',prop] if origin=='master' else None)
save('property-hierarchy', '<a:bodyPr lIns="11"/>',definition('spDef','<a:bodyPr rIns="7"/>')+definition('lnDef','<a:bodyPr rIns="8"/>')+definition('txDef','<a:bodyPr rIns="9"/>'),['<a:bodyPr tIns="22"/>','<a:bodyPr tIns="33" bIns="44"/>'])
save('exact-autofit',defs=definition('txDef','<a:bodyPr><a:normAutofit fontScale="92.00000000001%" lnSpcReduction="1.00000000001%"/></a:bodyPr>'))
save('local-no-autofit','<a:bodyPr><a:noAutofit/></a:bodyPr>',definition('txDef','<a:bodyPr><a:normAutofit fontScale="80000"/></a:bodyPr>'))
save('theme-list',defs=definition('txDef','<a:bodyPr lIns="19"/>','<a:lvl1pPr><a:defRPr sz="2400"/></a:lvl1pPr>'))
save('unknown-body','<a:bodyPr unknown="1"/>',status='unresolved')
save('wordart-body','<a:bodyPr><a:prstTxWarp prst="textArchUp"><a:avLst/></a:prstTxWarp></a:bodyPr>',status='unresolved')
save('unknown-theme',defs=definition('txDef','<a:bodyPr/>')+'<a:extLst/>',status='unresolved')
save('unknown-theme-entry',defs=definition('txDef','<a:bodyPr/>').replace('<a:txDef>','<a:txDef unknown="1">'),status='unresolved')
save('ignored-shape-sibling',defs=definition('txDef','<a:bodyPr/>').replace('<a:spPr/>','<a:spPr><a:bodyPr lIns="999"/></a:spPr>'))
save('mce-theme',defs=definition('txDef','<a:bodyPr><mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice Requires="a"><a:spAutoFit/></mc:Choice><mc:Fallback><a:noAutofit/></mc:Fallback></mc:AlternateContent></a:bodyPr>'))
save('bad-body','<a:bodyPr numCol="17"/>',status='error')
save('duplicate-theme',defs=definition('txDef','<a:bodyPr/>')+definition('txDef','<a:bodyPr/>'),status='error')
save('duplicate-choice',defs=definition('txDef','<a:bodyPr><a:noAutofit/><a:spAutoFit/></a:bodyPr>'),status='error')
(ROOT/'manifest.json').write_text(json.dumps({'format':'musteroffice.text-body-fixtures/1','previousEvidenceSha256':sha(PREVIOUS.read_bytes()),'cases':cases},indent=2)+'\n');print(json.dumps({'packages':len(cases)}))
