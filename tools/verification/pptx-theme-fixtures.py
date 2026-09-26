"""Owned theme declarations/overrides; independent ZIP/XML producer."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import A, MC

NS = {'a': A}
BASE = 'ppt/theme/theme2.xml'


def encode(root):
    return E.tostring(root, xml_declaration=True, encoding='UTF-8')


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('manifest', type=Path); parser.add_argument('directory', type=Path)
    args = parser.parse_args(); args.directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(args.manifest.read_text())
    def load(name):
        case = next(c for c in manifest['cases'] if c['name'] == name)
        with zipfile.ZipFile(case['path']) as z: return {n:z.read(n) for n in z.namelist()}
    authored = load('authored')
    overrides = load('theme-override-chain')
    original = E.fromstring(authored[BASE])
    def emit(name, parts, error=None, **extra):
        path = args.directory / (name+'.pptx')
        with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
            for key,value in parts.items():
                info=zipfile.ZipInfo(key,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,value)
        expect = {'error':error} if error else {'objects':15,'edit':'success'}
        manifest['cases'].append({'name':name,'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'expect':expect|extra})
    def color(root, slot, tag, attributes):
        element = root.find('a:themeElements/a:clrScheme/a:'+slot,NS)
        for child in list(element): element.remove(child)
        return E.SubElement(element, E.QName(A,tag), attributes)
    parts=copy.deepcopy(authored); root=copy.deepcopy(original)
    transforms=[('tint','25%'),('shade','75000'),('comp',None),('inv',None),('gray',None),('alpha','50%'),('alphaOff','-25000'),('alphaMod','200000'),
                ('hue','12000000'),('hueOff','-60000'),('hueMod','150%'),('sat','50.123456789%'),('satOff','-20000'),('satMod','200000'),
                ('lum','50000'),('lumOff','-50%'),('lumMod','125000'),('red','25000'),('redOff','-1000'),('redMod','100000'),
                ('green','25000'),('greenOff','1000'),('greenMod','100000'),('blue','25000'),('blueOff','1000'),('blueMod','100000'),('gamma',None),('invGamma',None),('alpha','75000')]
    rgb = color(root,'accent1','srgbClr',{'val':'12abEF'})
    for name,value in transforms: E.SubElement(rgb,E.QName(A,name),{} if value is None else {'val':value})
    color(root,'accent2','scrgbClr',{'r':'50.123456789%','g':'+0001','b':'-20000'})
    color(root,'accent3','hslClr',{'hue':'60000','sat':'150%','lum':'-20000'})
    color(root,'accent4','sysClr',{'val':'windowText','lastClr':'1a2B3c'})
    color(root,'accent5','prstClr',{'val':'aliceBlue'})
    color(root,'accent6','schemeClr',{'val':'dk1'})
    for group in ['majorFont','minorFont']:
        fonts=root.find('a:themeElements/a:fontScheme/a:'+group,NS)
        fonts.find('a:ea',NS).set('typeface','')
        fonts.find('a:cs',NS).set('typeface','')
        fonts.find('a:latin',NS).attrib.update({'panose':'020B0604020202020204','pitchFamily':'34','charset':'-128'})
        for script,face in [('Hans','Owned CJK Serif'),('Arab','Owned Arabic'),('Hans','Owned Duplicate CJK')]:
            E.SubElement(fonts,E.QName(A,'font'),script=script,typeface=face)
    parts[BASE]=encode(root); emit('theme-all-color-models',parts)
    utf=copy.deepcopy(parts); utf[BASE]=E.tostring(root,encoding='UTF-16',xml_declaration=True)
    emit('theme-utf16',utf)

    parts=copy.deepcopy(overrides)
    layout=E.Element(E.QName(A,'themeOverride'),nsmap={'a':A}); scheme=copy.deepcopy(original.find('a:themeElements/a:clrScheme',NS));scheme.set('name','Layout Colors')
    scheme.find('a:accent1/a:srgbClr',NS).set('val','FF0000');layout.append(scheme)
    slide=E.Element(E.QName(A,'themeOverride'),nsmap={'a':A}); fonts=copy.deepcopy(original.find('a:themeElements/a:fontScheme',NS));fonts.set('name','Slide Fonts')
    fonts.find('a:majorFont/a:latin',NS).set('typeface','Owned Heading');slide.append(fonts)
    parts['ppt/theme/override1.xml']=encode(layout);parts['ppt/theme/override2.xml']=encode(slide)
    emit('theme-independent-overrides',parts,themeSelection={'colors':'/ppt/theme/override1.xml','fonts':'/ppt/theme/override2.xml','format':'/ppt/theme/theme2.xml'})
    other=copy.deepcopy(parts); slide.insert(0,copy.deepcopy(scheme));other['ppt/theme/override2.xml']=encode(slide)
    emit('theme-nearest-color-override',other,themeSelection={'colors':'/ppt/theme/override2.xml','fonts':'/ppt/theme/override2.xml','format':'/ppt/theme/theme2.xml'})

    parts=copy.deepcopy(authored); root=copy.deepcopy(original); colors=root.find('a:themeElements/a:clrScheme',NS)
    selected=colors.find('a:accent1',NS);pos=list(colors).index(selected);colors.remove(selected)
    alt=E.Element(E.QName(MC,'AlternateContent'),nsmap={'mc':MC,'u':'urn:owned:future'})
    bad=E.SubElement(E.SubElement(alt,E.QName(MC,'Choice'),Requires='u'),E.QName(A,'accent1'));E.SubElement(bad,E.QName(A,'srgbClr'),val='notRGB')
    E.SubElement(alt,E.QName(MC,'Fallback')).append(selected);colors.insert(pos,alt)
    opaque=E.Element('{urn:owned}metadata');nested=E.SubElement(opaque,E.QName(A,'clrScheme'));E.SubElement(nested,E.QName(A,'accent1')).text='retained unknown payload'
    root.insert(0,opaque);parts[BASE]=encode(root);emit('theme-mce-retained',parts)

    for name,tag,attrs in [
        ('theme-invalid-rgb','srgbClr',{'val':'12345G'}),
        ('theme-invalid-preset','prstClr',{'val':'madeUp'}),
        ('theme-invalid-system','sysClr',{'val':'madeUp'}),
        ('theme-invalid-hue','hslClr',{'hue':'21600000','sat':'100%','lum':'100%'}),
        ('theme-invalid-percent','scrgbClr',{'r':'1e2%','g':'0','b':'0'}),
    ]:
        parts=copy.deepcopy(authored);root=copy.deepcopy(original);color(root,'accent1',tag,attrs);parts[BASE]=encode(root);emit(name,parts,'INPUT_INVALID')
    for name,transform,value in [('theme-alpha-range','alpha','100001'),('theme-alpha-decimal-range','alpha','100.99%'),('theme-negative-alpha-mod','alphaMod','-1')]:
        parts=copy.deepcopy(authored);root=copy.deepcopy(original);c=color(root,'accent1','srgbClr',{'val':'000000'});E.SubElement(c,E.QName(A,transform),val=value)
        parts[BASE]=encode(root);emit(name,parts,'INPUT_INVALID')
    for name in ['theme-missing-color','theme-duplicate-font','theme-short-panose','theme-invalid-pitch','theme-invalid-charset','theme-too-few-styles','theme-wrong-root']:
        parts=copy.deepcopy(authored);root=copy.deepcopy(original)
        if name=='theme-missing-color':
            colors=root.find('a:themeElements/a:clrScheme',NS);colors.remove(colors.find('a:accent1',NS))
        elif name=='theme-duplicate-font':
            fonts=root.find('a:themeElements/a:fontScheme/a:majorFont',NS);fonts.append(copy.deepcopy(fonts.find('a:latin',NS)))
        elif name in ['theme-short-panose','theme-invalid-pitch','theme-invalid-charset']:
            attr,value={'theme-short-panose':('panose','1234'),'theme-invalid-pitch':('pitchFamily','3'),'theme-invalid-charset':('charset','128')}[name]
            root.find('a:themeElements/a:fontScheme/a:majorFont/a:latin',NS).set(attr,value)
        elif name=='theme-too-few-styles':
            styles=root.find('a:themeElements/a:fmtScheme/a:fillStyleLst',NS);styles.remove(styles[0])
        else: root.tag=E.QName(A,'themeOverride')
        parts[BASE]=encode(root);emit(name,parts,'INPUT_INVALID')
    (args.directory/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps({'cases':len(manifest['cases']),'manifest':str(args.directory/'manifest.json')}))


if __name__=='__main__': main()
