"""Independent XML/MCE, attribute, parent/ordinal and actual edit-byte checks."""
import hashlib,json,struct,zipfile
from pathlib import Path
from xml.sax.saxutils import escape
from lxml import etree as E
from mce_reference import A,P,R,project
ROOT=Path('.codex-work/source-text');N={'a':A,'p':P}
read=lambda p:json.loads(Path(p).read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
schema=E.XMLSchema(E.parse('.codex-work/ecma376/xsd/pml.xsd',E.XMLParser(resolve_entities=False,no_network=True)))
counts={'catalogs':0,'nodes':0,'attributes':0,'retainedLocations':0,'textLeaves':0,'editedPackages':0,'unchangedCompressedParts':0,'validModifiedParts':0,'invalidModifiedParts':0,'validModifiedStyleParts':0};cases=[];invalid=[]
boolean=lambda v:v.strip() in ['1','true','on']
lex=lambda v:v.strip()
def point(v):return {'kind':'universalMeasure','value':v.strip()} if v[-1].isalpha() else {'kind':'hundredthPoints','value':int(v)}
def attrs(spec):return {native:(name,conv) for native,name,conv in spec}
BODY=attrs([('rot','rotation',int),('spcFirstLastPara','paragraphSpacing',boolean),('vertOverflow','verticalOverflow',lex),('horzOverflow','horizontalOverflow',lex),('vert','vertical',lex),('wrap','wrap',lex),('lIns','leftInset',lex),('tIns','topInset',lex),('rIns','rightInset',lex),('bIns','bottomInset',lex),('numCol','columns',int),('spcCol','columnSpacing',lex),('rtlCol','rightToLeftColumns',boolean),('fromWordArt','fromWordArt',boolean),('anchor','anchor',lex),('anchorCtr','centerAnchor',boolean),('forceAA','forceAntialiasing',boolean),('upright','upright',boolean),('compatLnSpc','compatibleLineSpacing',boolean)])
PARAGRAPH=attrs([('marL','leftMargin',int),('marR','rightMargin',int),('lvl','level',int),('indent','indent',int),('algn','alignment',lex),('defTabSz','defaultTabSize',lex),('rtl','rightToLeft',boolean),('eaLnBrk','eastAsianLineBreak',boolean),('fontAlgn','fontAlignment',lex),('latinLnBrk','latinLineBreak',boolean),('hangingPunct','hangingPunctuation',boolean)])
CHARACTER=attrs([('kumimoji','kumimoji',boolean),('lang','language',str),('altLang','alternativeLanguage',str),('sz','size',int),('b','bold',boolean),('i','italic',boolean),('u','underline',lex),('strike','strike',lex),('kern','kerning',int),('cap','caps',lex),('spc','spacing',point),('normalizeH','normalizeHeight',boolean),('baseline','baseline',lex),('noProof','noProof',boolean),('dirty','dirty',boolean),('err','error',boolean),('smtClean','smartClean',boolean),('smtId','smartId',int),('bmk','bookmark',str)])
HYPERLINK=attrs([(f'{{{R}}}id','relationshipId',str),('invalidUrl','invalidUrl',str),('action','action',str),('tgtFrame','targetFrame',str),('tooltip','tooltip',str),('history','history',boolean),('highlightClick','highlightClick',boolean),('endSnd','endSound',boolean)])
def attributes(raw,got,spec):
    expected={field:None if raw.get(native) is None else conv(raw.get(native)) for native,(field,conv) in spec.items()}
    assert got==expected,(raw.tag,got,expected)
    counts['attributes']+=sum(raw.get(n) is not None for n in spec)
def values(raw,value,ordinal):
    kind=value['kind']
    if kind in ['body','paragraph','character','hyperlink']:attributes(raw,value['attributes'],{'body':BODY,'paragraph':PARAGRAPH,'character':CHARACTER,'hyperlink':HYPERLINK}[kind])
    elif kind=='font':attributes(raw,value['font'],attrs([('typeface','typeface',str),('panose','panose',lex),('pitchFamily','pitchFamily',int),('charset','charset',int)]))
    elif kind=='autofit':attributes(raw,{k:v for k,v in value.items() if k!='kind'},attrs([('fontScale','fontScale',lex),('lnSpcReduction','lineSpacingReduction',lex)]))
    elif kind in ['percentage','points']:assert value['value']==(lex(raw.get('val')) if kind=='percentage' else int(raw.get('val')));counts['attributes']+=1
    elif kind=='autoNumber':attributes(raw,{k:v for k,v in value.items() if k!='kind'},attrs([('type','scheme',lex),('startAt','startAt',int)]))
    elif kind=='bulletCharacter':assert value['character']==raw.get('char');counts['attributes']+=1
    elif kind=='tab':attributes(raw,{k:v for k,v in value.items() if k!='kind'},attrs([('pos','position',lex),('algn','alignment',lex)]))
    elif kind=='field':assert value['id']==raw.get('id') and value['fieldType']==raw.get('type');counts['attributes']+=len(raw.attrib)
    elif kind=='fontReference':assert value['index']==raw.get('idx');counts['attributes']+=1
    elif kind=='rightToLeft':assert value['value']==(None if raw.get('val') is None else boolean(raw.get('val')));counts['attributes']+=len(raw.attrib)
    elif kind=='color':
        c=value['color'];assert c['sourceOrdinal']==ordinal
        if E.QName(raw).localname=='srgbClr':assert c['value']=={'kind':'srgb','rgb':list(bytes.fromhex(raw.get('val')))}
        elif E.QName(raw).localname=='schemeClr':assert c['value']=={'kind':'scheme','slot':raw.get('val')}
        else:raise AssertionError('extend the independent color oracle for this corpus')
        assert len(c['transforms'])==len(raw)
        for t,r in zip(c['transforms'],raw,strict=True):
            assert t['kind']==E.QName(r).localname
            if r.get('val') is not None:assert t['value']==r.get('val')
        counts['attributes']+=len(raw.attrib)+sum(len(r.attrib) for r in raw)
    elif kind=='line':
        assert value['line']['sourceOrdinal']==ordinal
        assert value['line']['width']==(None if raw.get('w') is None else str(int(raw.get('w'))))
    elif kind=='fill':assert value['fill']['sourceOrdinal']==ordinal
    elif kind=='effects':assert value['effects']['sourceOrdinal']==ordinal
    else:assert kind=='container'
def catalog(raw,catalog):
    root,_,ordinals=project(raw,True);physical={i:n for i,n in enumerate(E.fromstring(raw).iter())};projected={o:n for n,o in ordinals.items()};nodes={int(k):v for k,v in catalog['nodes'].items()}
    counts['catalogs']+=1
    assert [r['sourceOrdinal'] for r in catalog['roots']]==[i for i,n in nodes.items() if n['parent'] is None]
    for ident,n in nodes.items():
        p=projected[ident];assert E.QName(p).localname==n['element']
        ancestor=next((ordinals[a] for a in p.iterancestors() if ordinals.get(a) in nodes),None);assert ancestor==n['parent']
        assert n['children']==[i for i,c in nodes.items() if c['parent']==ident]
        values(p,n['value'],ident);counts['nodes']+=1
        for at in n['retainedOrdinals']:
            owned=physical[ident];unknown=physical[at];assert unknown is owned or owned in unknown.iterancestors();counts['retainedLocations']+=1
    for r in catalog['roots']:
        n=projected[r['sourceOrdinal']];shape=next((a for a in n.iterancestors() if E.QName(a).namespace==P and E.QName(a).localname in ['sp','pic','cxnSp','grpSp','graphicFrame']),None)
        expected=None if shape is None else int(shape.find('./*/p:cNvPr',N).get('id'));assert r['owner']==expected
    # All independently projected semantic roots must appear, including those
    # with no properties. Unknown extension content cannot inject extra roots.
    expected_roots=[]
    for n in root.iter():
        q=E.QName(n);parent=n.getparent()
        if parent is not None and ((q.namespace==P and q.localname in ['defaultTextStyle','txStyles','txBody']) or (q.namespace==A and q.localname=='fontRef' and parent.tag==f'{{{P}}}style')):
            expected_roots.append(ordinals[n])
    assert expected_roots==[r['sourceOrdinal'] for r in catalog['roots']]
    return root
def compressed(blob,info):
    names,extra=struct.unpack_from('<HH',blob,info.header_offset+26);start=info.header_offset+30+names+extra;return blob[start:start+info.compress_size]
for c in read(ROOT/'manifest.json')['cases']:
    b=Path(c['path']).read_bytes();assert sha(b)==c['sha256'];response_path=ROOT/(c['name']+'.response.json');response=read(response_path)
    with zipfile.ZipFile(c['path']) as z:
        if c['name'].startswith('scopes-'):
            for part in ['ppt/presentation.xml','ppt/slideMasters/slideMaster2.xml']:
                style,_=project(z.read(part));assert schema.validate(style),(c['name'],part,str(schema.error_log));counts['validModifiedStyleParts']+=1
        xml=z.read('ppt/slides/slide1.xml');projected,_=project(xml)
        valid=schema.validate(projected)
        assert valid==c['xsd'],(c['name'],str(schema.error_log))
        counts['validModifiedParts' if valid else 'invalidModifiedParts']+=1
        if not valid:invalid.append({'name':c['name'],'reason':str(schema.error_log.last_error)})
        if c['error']:assert response['status']=='error' and response['error']['code']=='INPUT_INVALID';continue
        index=response['index']
        if index.get('text'):catalog(z.read(index['mainPart'][1:]),index['text'])
        for part,surface in index['surfaces'].items():
            if not surface.get('text'):continue
            root=catalog(z.read(part[1:]),surface['text'])
            objects={int(n.find('./*/p:cNvPr',N).get('id')):n for n in root.findall('.//p:sp',N)}
            for obj in surface['objects']:
                if obj['kind']!='shape' or obj.get('textBodyOrdinal') is None:continue
                paragraphs=objects[obj['nativeId']].findall('p:txBody/a:p',N);assert len(paragraphs)==len(obj['paragraphs'])
                for raw_p,paragraph in zip(paragraphs,obj['paragraphs'],strict=True):
                    runs=[n for n in raw_p if n.tag in [f'{{{A}}}'+k for k in ['r','br','fld']]];assert len(runs)==len(paragraph)
                    for raw_r,run in zip(runs,paragraph,strict=True):
                        assert run['kind']=={'r':'text','br':'break','fld':'field'}[E.QName(raw_r).localname]
                        t=raw_r.find('a:t',N);assert run['text']==('' if t is None else ''.join(t.itertext()));counts['textLeaves']+=1
        if c['editable']:
            candidate=ROOT/(c['name']+'.edited.pptx');out=candidate.read_bytes();q=read(ROOT/(c['name']+'.edit.json'))
            with zipfile.ZipFile(candidate) as edited:
                assert z.namelist()==edited.namelist();request=q['edits'][0]
                before=escape(request['expectedText']).encode();after=escape(request['replacement']).encode();assert xml.count(before)==1
                assert edited.read('ppt/slides/slide1.xml')==xml.replace(before,after,1)
                for info in z.infolist():
                    if info.filename=='ppt/slides/slide1.xml':continue
                    assert z.read(info.filename)==edited.read(info.filename)
                    assert compressed(b,info)==compressed(out,edited.getinfo(info.filename));counts['unchangedCompressedParts']+=1
                changed,_=project(edited.read('ppt/slides/slide1.xml'));assert schema.validate(changed)==c['xsd']
                counts['validModifiedParts' if c['xsd'] else 'invalidModifiedParts']+=1
            counts['editedPackages']+=1
    cases.append({'name':c['name'],'sourcePath':c['path'],'sourceSha256':c['sha256'],'responsePath':str(response_path),'responseSha256':sha(response_path.read_bytes())})
result={'format':'musteroffice.source-text-reference/1','counts':counts,'invalidModifiedParts':invalid,'scope':'Independent projected XML ancestry, physical ordinals, explicit text attributes, selected colors/fonts and unchanged source bytes. Source declarations only; not inheritance, font resolution, layout, rendering or Office/WPS acceptance. XSD checks the changed slide parts, after independent MCE projection.','cases':cases}
(ROOT/'reference.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(counts))
