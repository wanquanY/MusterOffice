"""Owned source text declarations, native enum facts and malformed boundaries."""
import copy,hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P
ROOT=Path('.codex-work/source-text');ROOT.mkdir(exist_ok=True);N={'a':A,'p':P};SLIDE='ppt/slides/slide1.xml'
sha=lambda b:hashlib.sha256(b).hexdigest()
base=next(c for c in json.loads(Path('.codex-work/source-page/manifest.json').read_text())['cases'] if c['name']=='visible-text')
assert sha(Path(base['path']).read_bytes())==base['sha256']
with zipfile.ZipFile(base['path']) as z:BASE={n:z.read(n) for n in z.namelist()}
cases=[]
def body(bp='',pp='',rp='',bp_children='',pp_children='',rp_children='',extra='',text='Owned 中ع e\u0301',list_style=''):
    return f'<a:bodyPr {bp}>{bp_children}</a:bodyPr>{list_style}<a:p><a:pPr {pp}>{pp_children}</a:pPr><a:r><a:rPr {rp}>{rp_children}</a:rPr><a:t>{text}</a:t></a:r>{extra}</a:p>'
def emit(name,content,error=False,editable=True,xsd=True,root_attrs=None,mutate=None):
    parts=dict(BASE);root=E.fromstring(parts[SLIDE]);shape=root.find('p:cSld/p:spTree/p:sp',N);tx=shape.find('p:txBody',N)
    replacement=E.fromstring(f'<p:txBody xmlns:p="{P}" xmlns:a="{A}">{content}</p:txBody>')
    shape.replace(tx,replacement)
    if root_attrs:
        for k,v in root_attrs.items():root.set(k,v)
    parts[SLIDE]=E.tostring(root,xml_declaration=True,encoding='utf-8')
    if mutate:mutate(parts)
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w') as z:
        for n,b in parts.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    cases.append({'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'error':error,'editable':editable and not error,'xsd':xsd})
emit('empty-properties',body())
emit('all-body-attributes',body(bp='rot="-21600001" spcFirstLastPara="1" vertOverflow="ellipsis" horzOverflow="clip" vert="vert270" wrap="none" lIns="-0.00000000001mm" tIns="+000" rIns="2147483647" bIns="-2147483648" numCol="16" spcCol="0" rtlCol="0" fromWordArt="false" anchor="ctr" anchorCtr="1" forceAA="true" upright="0" compatLnSpc="false"',bp_children='<a:normAutofit fontScale="92.000000001%" lnSpcReduction="0%"/>'))
emit('all-paragraph-attributes',body(pp='marL="0" marR="51206400" lvl="8" indent="-51206400" algn="just" defTabSz="1.23456789pt" rtl="0" eaLnBrk="1" fontAlgn="base" latinLnBrk="false" hangingPunct="true"',pp_children='<a:lnSpc><a:spcPct val="13200000"/></a:lnSpc><a:spcBef><a:spcPts val="0"/></a:spcBef><a:spcAft><a:spcPct val="110.00000001%"/></a:spcAft><a:buNone/><a:tabLst><a:tab/><a:tab pos="-2147483648" algn="dec"/></a:tabLst><a:defRPr sz="2000"/>'))
emit('all-character-attributes',body(rp='kumimoji="true" lang="zh-CN" altLang="ar-SA" sz="1234" b="0" i="true" u="wavyDbl" strike="dblStrike" kern="0" cap="small" spc="-0.000000001pt" normalizeH="false" baseline="-20.00000001%" noProof="0" dirty="false" err="true" smtClean="0" smtId="4294967295" bmk=" Preserve spaces "',rp_children='<a:latin typeface="+mn-lt" panose="00000000000000000000" pitchFamily="00" charset="-128"/><a:ea typeface=""/><a:cs typeface="Explicit complex"/><a:sym typeface="Symbol"/><a:hlinkClick action="ppaction://hlinkshowjump" tooltip=" Keep spaces " history="0"/><a:rtl val="on"/>'))
schema=E.parse('.codex-work/ecma376/xsd/dml-main.xsd');X={'x':'http://www.w3.org/2001/XMLSchema'}
for type_,field,where in [('ST_TextAnchoringType','anchor','bp'),('ST_TextVertOverflowType','vertOverflow','bp'),('ST_TextHorzOverflowType','horzOverflow','bp'),('ST_TextVerticalType','vert','bp'),('ST_TextWrappingType','wrap','bp'),('ST_TextUnderlineType','u','rp'),('ST_TextStrikeType','strike','rp'),('ST_TextCapsType','cap','rp'),('ST_TextAlignType','algn','pp'),('ST_TextFontAlignType','fontAlgn','pp')]:
    for item in schema.findall(f"x:simpleType[@name='{type_}']/x:restriction/x:enumeration",X):
        value=item.get('value');emit(f'enum-{field}-{value}',body(**{where:f'{field}="{value}"'}))
for item in schema.findall("x:simpleType[@name='ST_TextAutonumberScheme']/x:restriction/x:enumeration",X):
    value=item.get('value');emit('number-'+value,body(pp_children=f'<a:buAutoNum type="{value}"/>'))
for v in ['noAutofit','spAutoFit','normAutofit']:emit(v,body(bp_children=f'<a:{v}/>'))
for v in ['true','false','1','0','on','off']:emit('rtl-'+v,body(rp_children=f'<a:rtl val="{v}"/>'))
for name,child in [('bullet-follow','<a:buClrTx/><a:buSzTx/><a:buFontTx/><a:buChar char="●"/>'),('bullet-explicit','<a:buClr><a:srgbClr val="123456"><a:alpha val="50%"/></a:srgbClr></a:buClr><a:buSzPct val="100%"/><a:buFont typeface="Bullet"/><a:buAutoNum type="alphaLcParenBoth" startAt="32767"/>'),('bullet-points','<a:buSzPts val="100"/><a:buNone/>')]:emit(name,body(pp_children=child))
emit('tabs-32',body(pp_children='<a:tabLst>'+''.join(f'<a:tab pos="{i*12700}" algn="{["l","ctr","r","dec"][i%4]}"/>' for i in range(32))+'</a:tabLst>'))
emit('list-levels',body(list_style='<a:lstStyle><a:defPPr algn="l"/>'+''.join(f'<a:lvl{i}pPr lvl="{i-1}"><a:defRPr sz="{i*100}"/></a:lvl{i}pPr>' for i in range(1,10))+'</a:lstStyle>',extra='<a:br><a:rPr sz="1200"/></a:br><a:endParaRPr sz="1800"/>'))
emit('field',body(extra='<a:fld id="{00000000-0000-0000-0000-000000000000}" type="slidenum"><a:rPr sz="1500"/><a:pPr lvl="2"/><a:t>7</a:t></a:fld>'))
emit('paint',body(rp_children='<a:ln w="12700"><a:solidFill><a:srgbClr val="123456"/></a:solidFill></a:ln><a:gradFill><a:gsLst><a:gs pos="0"><a:schemeClr val="accent1"/></a:gs><a:gs pos="100000"><a:srgbClr val="FFFFFF"/></a:gs></a:gsLst></a:gradFill><a:effectLst><a:glow rad="500"><a:srgbClr val="000000"/></a:glow></a:effectLst><a:highlight><a:srgbClr val="FEDCBA"><a:alpha val="50%"/><a:tint val="20000"/></a:srgbClr></a:highlight><a:uLn w="0"><a:noFill/></a:uLn><a:uFill><a:solidFill><a:srgbClr val="AABBCC"/></a:solidFill></a:uFill>'))
emit('retained-wordart',body(bp_children='<a:prstTxWarp prst="textWave1"><a:avLst/></a:prstTxWarp><a:spAutoFit/>'))
emit('retained-extension',body(rp_children='<a:extLst><a:ext uri="owned"><a:latin typeface="decoy"/></a:ext></a:extLst>'))
emit('retained-attribute',body(bp='unknown="retained"'),xsd=False)
MC='http://schemas.openxmlformats.org/markup-compatibility/2006'
emit('mce-body-autofit',body(bp_children=f'<mc:AlternateContent xmlns:mc="{MC}"><mc:Choice Requires="a"><a:normAutofit fontScale="80%"/></mc:Choice><mc:Fallback><a:normAutofit fontScale="not-active"/></mc:Fallback></mc:AlternateContent>'))
emit('mce-font-choice',body(rp_children=f'<mc:AlternateContent xmlns:mc="{MC}" xmlns:unused="urn:owned:unknown"><mc:Choice Requires="unused"><a:latin typeface="inactive"/></mc:Choice><mc:Fallback><a:latin typeface="Active"/></mc:Fallback></mc:AlternateContent>'))
def scopes(index):
    def mutate(parts):
        for part,tag,content in [('ppt/presentation.xml','defaultTextStyle','<a:defPPr algn="r"><a:defRPr sz="1000"/></a:defPPr>'),('ppt/slideMasters/slideMaster2.xml','txStyles','<p:titleStyle><a:lvl1pPr algn="ctr"/></p:titleStyle><p:bodyStyle/><p:otherStyle/>')]:
            root=E.fromstring(parts[part]);old=root.find('p:'+tag,N);new=E.fromstring(f'<p:{tag} xmlns:p="{P}" xmlns:a="{A}">{content}</p:{tag}>')
            if old is None:root.append(new)
            else:root.replace(old,new)
            parts[part]=E.tostring(root,xml_declaration=True,encoding='utf-8')
        root=E.fromstring(parts[SLIDE]);shape=root.find('p:cSld/p:spTree/p:sp',N);style=E.fromstring(f'<p:style xmlns:p="{P}" xmlns:a="{A}"><a:lnRef idx="0"/><a:fillRef idx="0"/><a:effectRef idx="0"/><a:fontRef idx="{index}"><a:schemeClr val="tx1"/></a:fontRef></p:style>');shape.insert(2,style);parts[SLIDE]=E.tostring(root,xml_declaration=True,encoding='utf-8')
    return mutate
for index in ['none','major','minor']:emit('scopes-'+index,body(),mutate=scopes(index))
for name,where,attribute in [('columns','bp','numCol="17"'),('negative-columns','bp','numCol="0"'),('coordinate32','bp','lIns="2147483648"'),('spacing32','bp','spcCol="-1"'),('body-enum','bp','anchor="middle"'),('bool','rp','b="yes"'),('font-small','rp','sz="99"'),('font-large','rp','sz="400001"'),('kerning','rp','kern="-1"'),('level','pp','lvl="9"'),('margin','pp','marL="-1"'),('indent','pp','indent="51206401"'),('text-point','rp','spc="400001"'),('percent','rp','baseline="NaN%"')]:emit('bad-'+name,body(**{where:attribute}),error=True,xsd=False)
for name,content in [('autofit-choice',body(bp_children='<a:noAutofit/><a:normAutofit/>')),('spacing-empty',body(pp_children='<a:lnSpc/>')),('spacing-choice',body(pp_children='<a:lnSpc><a:spcPct val="100%"/><a:spcPts val="100"/></a:lnSpc>')),('font-repeat',body(rp_children='<a:latin typeface="a"/><a:latin typeface="b"/>')),('bullet-choice',body(pp_children='<a:buNone/><a:buChar char="x"/>')),('run-order','<a:bodyPr/><a:p><a:r><a:t>A</a:t><a:rPr/></a:r></a:p>'),('tabs-33',body(pp_children='<a:tabLst>'+'<a:tab/>'*33+'</a:tabLst>'))]:emit('bad-'+name,content,error=True,xsd=False)
(ROOT/'manifest.json').write_text(json.dumps({'base':base,'cases':cases},indent=2)+'\n');print(json.dumps({'packages':len(cases),'valid':sum(not c['error'] for c in cases),'negative':sum(c['error'] for c in cases)}))
