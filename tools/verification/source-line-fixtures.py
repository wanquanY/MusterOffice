"""Owned native line inputs produced by independent ZIP/XML tooling."""
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import A,P,MC

ROOT=Path('.codex-work/source-lines');ROOT.mkdir(parents=True,exist_ok=True)
manifest=json.loads(Path('.codex-work/pptx-color-map-fixtures/manifest.json').read_text())
base=next(c for c in manifest['cases'] if c['name']=='authored')
assert hashlib.sha256(Path(base['path']).read_bytes()).hexdigest()==base['sha256']
with zipfile.ZipFile(base['path']) as z: parts={n:z.read(n) for n in z.namelist()}
cases=[];NS={'a':A,'p':P}
def emit(name,line='',reference=None,error=None,theme=False,conforming=True,utf16=False):
    owned=copy.deepcopy(parts);part='ppt/theme/theme2.xml' if theme else 'ppt/slides/slide1.xml'
    original=E.fromstring(owned[part])
    # Requires contains lexical QNames; lxml cannot infer that this namespace
    # must survive moving a subtree out of the temporary wrapper.
    root=E.Element(original.tag,nsmap={**original.nsmap,'mc':MC,'u':'urn:owned:future'})
    root.attrib.update(original.attrib);root.text=original.text
    for child in original:root.append(child)
    wrapper=E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}" xmlns:mc="{MC}" xmlns:u="urn:owned:future">{line}</root>')
    if theme:
        styles=root.find('a:themeElements/a:fmtScheme/a:lnStyleLst',NS);styles.replace(styles[0],wrapper[0])
    else:
        shape=root.find('p:cSld/p:spTree/p:sp',NS);props=shape.find('p:spPr',NS)
        for node in list(props): props.remove(node)
        for node in list(wrapper):props.append(node)
        old=shape.find('p:style',NS)
        if old is not None:shape.remove(old)
        if reference is not None:
            style=E.fromstring(f'<p:style xmlns:p="{P}" xmlns:a="{A}">{reference}<a:fillRef idx="0"><a:schemeClr val="phClr"/></a:fillRef><a:effectRef idx="0"><a:schemeClr val="phClr"/></a:effectRef><a:fontRef idx="minor"><a:schemeClr val="tx1"/></a:fontRef></p:style>')
            shape.insert(list(shape).index(props)+1,style)
    owned[part]=E.tostring(root,xml_declaration=True,encoding='UTF-16' if utf16 else 'UTF-8')
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for n,b in owned.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    cases.append({'name':name,'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'part':'/'+part,'error':error,'xsdConforming':conforming and error is None})

emit('absent');emit('empty','<a:ln/>')
for name,lim in [('absent',None),('zero','0'),('integer','400000'),('decimal','400.123456789%'),('lexical','+000400000')]:
    emit('miter-'+name,'<a:ln><a:miter'+('' if lim is None else f' lim="{lim}"')+'/></a:ln>')
for cap in ['flat','rnd','sq']:emit('cap-'+cap,f'<a:ln cap="{cap}"/>')
for cmpd in ['sng','dbl','thickThin','thinThick','tri']:emit('compound-'+cmpd,f'<a:ln cmpd="{cmpd}"/>')
for align in ['ctr','in']:emit('alignment-'+align,f'<a:ln algn="{align}"/>')
for width in ['0','12700','20116800']:emit('width-'+width,f'<a:ln w="{width}"/>')
for dash in ['solid','dot','dash','lgDash','dashDot','lgDashDot','lgDashDotDot','sysDash','sysDot','sysDashDot','sysDashDotDot']:
    emit('dash-'+dash,f'<a:ln><a:prstDash val="{dash}"/></a:ln>')
emit('dash-absent','<a:ln><a:prstDash/></a:ln>');emit('dash-empty','<a:ln><a:custDash/></a:ln>')
emit('dash-exact','<a:ln><a:custDash><a:ds d="12.345678901%" sp="0"/><a:ds d="+000001" sp="100000"/></a:custDash></a:ln>')
for i,end in enumerate(['none','triangle','stealth','diamond','oval','arrow']):
    size=['sm','med','lg'][i%3];emit('end-'+end,f'<a:ln><a:headEnd type="{end}" w="{size}" len="{size}"/><a:tailEnd/></a:ln>')
for name,color in [
 ('srgb','<a:srgbClr val="12abEF"/>'),('scrgb','<a:scrgbClr r="50.123456789%" g="+0001" b="-20000"/>'),
 ('hsl','<a:hslClr hue="60000" sat="150%" lum="-20000"/>'),('system','<a:sysClr val="windowText" lastClr="1a2B3c"/>'),
 ('preset','<a:prstClr val="aliceBlue"/>'),('scheme','<a:schemeClr val="accent3"><a:alpha val="50%"/><a:alpha val="+050000"/></a:schemeClr>')]:
    emit('color-'+name,f'<a:ln><a:solidFill>{color}</a:solidFill></a:ln>',f'<a:lnRef idx="2">{color}</a:lnRef>')
for fill in ['noFill','solidFill','gradFill','pattFill']:emit('fill-'+fill,f'<a:ln><a:{fill}/></a:ln>')
for index in ['0','1','3','4294967295']:emit('reference-'+index,'',f'<a:lnRef idx="{index}"/>')
complex_line='<a:ln w="12700" cap="sq" cmpd="dbl" algn="ctr"><a:solidFill><a:schemeClr val="phClr"><a:tint val="25%"/></a:schemeClr></a:solidFill><a:prstDash val="dashDot"/><a:miter/><a:headEnd type="arrow"/><a:tailEnd type="none"/></a:ln>'
emit('theme-line',complex_line,theme=True);emit('utf16-line',complex_line,utf16=True)
emit('selected-fallback','<mc:AlternateContent><mc:Choice Requires="u"><a:ln cap="invalid"/></mc:Choice><mc:Fallback><a:ln cap="flat"><mc:AlternateContent><mc:Choice Requires="u"><a:miter lim="bad"/></mc:Choice><mc:Fallback><a:miter/></mc:Fallback></mc:AlternateContent></a:ln></mc:Fallback></mc:AlternateContent>')
emit('retained-extension','<a:ln><a:round/><a:extLst><a:ext uri="owned"><u:data><a:ln cap="invalid"/></u:data></a:ext></a:extLst></a:ln>')
emit('retained-attribute','<a:ln owned="retained"><a:noFill/></a:ln>',conforming=False)
for name,line in [
 ('width-negative','<a:ln w="-1"/>'),('width-large','<a:ln w="20116801"/>'),('cap','<a:ln cap="round"/>'),('compound','<a:ln cmpd="single"/>'),('alignment','<a:ln algn="center"/>'),
 ('negative-limit','<a:ln><a:miter lim="-1"/></a:ln>'),('large-limit','<a:ln><a:miter lim="2147483648"/></a:ln>'),('exponent','<a:ln><a:miter lim="1e2%"/></a:ln>'),
 ('join-duplicate','<a:ln><a:round/><a:bevel/></a:ln>'),('order','<a:ln><a:round/><a:noFill/></a:ln>'),('dash-stop','<a:ln><a:custDash><a:ds d="1"/></a:custDash></a:ln>'),
 ('end','<a:ln><a:headEnd type="circle"/></a:ln>'),('dash','<a:ln><a:prstDash val="dashed"/></a:ln>'),('duplicate','<a:ln/><a:ln/>'),
 ('color','<a:ln><a:solidFill><a:srgbClr val="12345G"/></a:solidFill></a:ln>'),('color-duplicate','<a:ln><a:solidFill><a:srgbClr val="123456"/><a:srgbClr val="654321"/></a:solidFill></a:ln>')]:
    emit('reject-'+name,line,error='INPUT_INVALID')
emit('reject-reference','',reference='<a:lnRef idx="-1"/>',error='INPUT_INVALID')
emit('reject-unknown','<a:ln><a:madeUp/></a:ln>',error='MAPPING_NOT_IMPLEMENTED')
(ROOT/'manifest.json').write_text(json.dumps({'base':base,'cases':cases},indent=2)+'\n')
print(json.dumps({'cases':len(cases),'valid':sum(c['error'] is None for c in cases)}))
