"""Compare actual Native/WASM inspection to independent FontTools and binary header reads."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from fontTools.ttLib import TTFont, TTCollection
import fontTools
p=argparse.ArgumentParser();p.add_argument('report',type=Path);a=p.parse_args()
r=json.loads(a.report.read_text());checked=[]
for case in r['cases']:
    if case['response']['status']!='inspected':continue
    data=Path(case['source']).read_bytes();assert hashlib.sha256(data).hexdigest()==case['sourceSha256']
    q=json.loads(case['request']);got=case['response']['font'];font=TTFont(case['source'],fontNumber=q['faceIndex'],recalcTimestamp=False)
    assert got['faceCount']==(struct.unpack_from('>I',data,8)[0] if data[:4]==b'ttcf' else 1)
    assert got['faceIndex']==q['faceIndex'];assert got['byteLength']==str(len(data))
    assert got['sfntVersion']==int.from_bytes(font.sfntVersion.encode('latin1'),'big')
    assert got['unitsPerEm']==font['head'].unitsPerEm and got['glyphCount']==font['maxp'].numGlyphs
    assert got['fontRevision1616']==round(font['head'].fontRevision*65536)
    tables=[{'tag':tag,'offset':str(t.offset),'byteLength':str(t.length),'checksum':t.checkSum} for tag,t in sorted(font.reader.tables.items())]
    assert got['tables']==tables
    names=[]
    for n in font['name'].names:
        enc='utf-16-be' if n.platformID==0 or (n.platformID==3 and n.platEncID in [0,1,10]) else ('mac_roman' if (n.platformID,n.platEncID)==(1,0) else None)
        text=n.string.decode(enc,errors='strict') if enc else None
        names.append({'platformId':n.platformID,'encodingId':n.platEncID,'languageId':n.langID,'nameId':n.nameID,'text':text})
    assert got['names']==names and got['languageTags']==[]
    h=font['hhea'];v=font.get('vhea')
    assert got['metrics']=={'horizontalAscender':h.ascent,'horizontalDescender':h.descent,'horizontalLineGap':h.lineGap,
        'vertical':None if v is None else {'ascender':v.ascent,'descender':v.descent,'lineGap':v.lineGap}}
    o=font.get('OS/2')
    expected=None if o is None else {'version':o.version,'weightClass':o.usWeightClass,'widthClass':o.usWidthClass,'fsType':o.fsType,'fsSelection':o.fsSelection,
        'typoAscender':o.sTypoAscender,'typoDescender':o.sTypoDescender,'typoLineGap':o.sTypoLineGap,'winAscent':o.usWinAscent,'winDescent':o.usWinDescent}
    assert got['os2']==expected
    fv=font.get('fvar');axes=[];instances=[]
    if fv:
        axes=[{'tag':a.axisTag,'minimum1616':round(a.minValue*65536),'default1616':round(a.defaultValue*65536),'maximum1616':round(a.maxValue*65536),'flags':a.flags,'nameId':a.axisNameID} for a in fv.axes]
        # FontTools uses 0xffff for absent PS IDs; the binary instance size distinguishes absence.
        raw=font.getTableData('fvar');instance_size=struct.unpack_from('>H',raw,14)[0]
        instances=[{'subfamilyNameId':i.subfamilyNameID,'postscriptNameId':i.postscriptNameID if instance_size==6+len(axes)*4 else None,
            'flags':i.flags,'coordinates1616':[round(i.coordinates[a.axisTag]*65536) for a in fv.axes]} for i in fv.instances]
    assert got['axes']==axes,(case['name'],got['axes'],axes)
    assert got['instances']==instances
    special10=case['name']=='format-10'
    if special10:
        raw=font.getTableData('cmap');offset=struct.unpack_from('>I',raw,8)[0]
        form,reserved,length,language,start,count=struct.unpack_from('>HHIIII',raw,offset)
        assert (form,reserved,length,language)==(10,0,24,0)
        ids=struct.unpack_from('>'+str(count)+'H',raw,offset+20)
        mapping={start+i:font.getGlyphName(gid) for i,gid in enumerate(ids) if gid}
    else: mapping=font.getBestCmap()
    assert mapping is not None
    selected=got['cmap'];table=font['cmap'].tables[selected['recordIndex']]
    if not special10: assert table.cmap==mapping
    assert (table.platformID,table.platEncID,table.format)==(selected['platformId'],selected['encodingId'],selected['format'])
    uv_index=selected['variationRecordIndex'];uvs={} if uv_index is None else font['cmap'].tables[uv_index].uvsDict
    counts={}
    for query,actual in zip(q['characters'],got['coverage'],strict=True):
        cp=query['codepoint'];vs=query['variationSelector'];assert query==actual['character']
        if vs is None:
            glyph=mapping.get(cp);expected={'kind':'mapped','glyphId':font.getGlyphID(glyph)} if glyph is not None and font.getGlyphID(glyph)!=0 else {'kind':'missing'}
        else:
            variants=dict(uvs.get(vs,[]))
            if cp not in variants:expected={'kind':'unsupportedVariation'}
            else:
                glyph=variants[cp] or mapping.get(cp)
                expected={'kind':'mapped','glyphId':font.getGlyphID(glyph)} if glyph is not None and font.getGlyphID(glyph)!=0 else {'kind':'missing'}
        assert actual['outcome']==expected,(case['name'],query,actual,expected)
        counts[expected['kind']]=counts.get(expected['kind'],0)+1
    checked.append({'name':case['name'],'sha256':case['sourceSha256'],'tables':len(tables),'names':len(names),'axes':len(axes),'instances':len(instances),
        'cmapFormat':selected['format'],'queries':len(got['coverage']),'outcomes':counts})
print(json.dumps({'format':'musteroffice.font-independent/1','fonttoolsVersion':fontTools.__version__,'passed':len(checked),'queries':sum(c['queries'] for c in checked),'cases':checked},indent=2))
