"""Independent ZIP/XML check of declared chart text cascades; no font/raster claim.
Usage: python3 chart-text-oracle.py <cli> <cases.json> <new-report.json>
"""
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

spec = importlib.util.spec_from_file_location('label_oracle', Path(__file__).with_name('chart-label-oracle.py'))
labels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(labels)
A = '{http://schemas.openxmlformats.org/drawingml/2006/main}'
C = labels.C
CHAR = dict(kumimoji='kumimoji', language='lang', alternativeLanguage='altLang', size='sz', bold='b', italic='i', underline='u', strike='strike', kerning='kern', caps='cap', spacing='spc', normalizeHeight='normalizeH', baseline='baseline', noProof='noProof', dirty='dirty', error='err', smartClean='smtClean', smartId='smtId', bookmark='bmk')
PARA = dict(leftMargin='marL', rightMargin='marR', level='lvl', indent='indent', alignment='algn', defaultTabSize='defTabSz', rightToLeft='rtl', eastAsianLineBreak='eaLnBrk', fontAlignment='fontAlgn', latinLineBreak='latinLnBrk', hangingPunctuation='hangingPunct')
BOOL = {'kumimoji','bold','italic','normalizeHeight','noProof','dirty','error','smartClean','rightToLeft','eastAsianLineBreak','latinLineBreak','hangingPunctuation'}
INT = {'size','kerning','smartId','leftMargin','rightMargin','level','indent'}
CSLOTS = dict(line=['ln'], fill=['noFill','solidFill','gradFill','blipFill','pattFill','grpFill'], effects=['effectLst','effectDag'], highlight=['highlight'], underlineLine=['uLnTx','uLn'], underlineFill=['uFillTx','uFill'], latin=['latin'], eastAsian=['ea'], complexScript=['cs'], symbol=['sym'], click=['hlinkClick'], mouseOver=['hlinkMouseOver'], rightToLeft=['rtl'])
PSLOTS = dict(lineSpacing=['lnSpc'], spaceBefore=['spcBef'], spaceAfter=['spcAft'], bulletColor=['buClr','buClrTx'], bulletSize=['buSzTx','buSzPts','buSzPct'], bulletFont=['buFont','buFontTx'], bullet=['buNone','buChar','buAutoNum'], tabs=['tabLst'])

def check(source, request, response):
    labels.check(source, request, response)
    plan = response['labels']
    with zipfile.ZipFile(io.BytesIO(source)) as package:
        root = ET.fromstring(package.read(plan['chartPart'].lstrip('/')))
    nodes = list(root.iter()); ids = {n:i for i,n in enumerate(nodes)}
    parents = {c:p for p in nodes for c in p}
    def origin(n):
        p = n
        while p.tag not in {C+'txPr', C+'rich'}:
            p = parents[p]
        return dict(kind='chart', part=plan['chartPart'], bodySourceOrdinal=ids[p], sourceOrdinal=ids[n])
    def properties(chain, mapping, paragraph=None):
        attrs, origins = {}, {}
        for role, name in mapping.items():
            found = next((n for n in chain if name in n.attrib and (role != 'level' or parents[n] is paragraph)), None)
            if found is None:
                attrs[role] = None
                continue
            v = found.get(name)
            if role in BOOL: v = v in {'true','1'}
            elif role in INT: v = int(v)
            elif role == 'spacing':
                try: v = dict(kind='hundredthPoints', value=int(v))
                except ValueError: v = dict(kind='universalMeasure', value=v)
            attrs[role] = v
            origins[role] = origin(found)
        return attrs, origins
    def declarations(chain, slots):
        result = {}
        for role, names in slots.items():
            matches = [c for n in chain for c in n if c.tag in {A+v for v in names}]
            if matches:
                n = matches[0]
                result[role] = dict(element=n.tag[len(A):], origin=origin(n))
        return result
    def character(style, chain):
        chain = [n for n in chain if n is not None]
        attrs, origins = properties(chain, CHAR)
        assert style['attributes'] == attrs
        assert style['origins'] == origins
        assert style['declarations'] == declarations(chain, CSLOTS)
    used = set(); unique = set(); paragraphs = runs = styles = 0
    for label in plan['labels']:
        deleted = label['settings']['deleted']
        roots = list(label['settings']['textPropertyRoots'])
        if label['customTextSource'] is not None:
            rich = nodes[label['customTextSource']].find(C+'rich')
            if rich is not None: roots.insert(0,ids[rich])
        if not roots or deleted is not None and deleted['value']:
            assert label['textCascade'] is None
            continue
        idx = label['textCascade']; used.add(idx)
        value = plan['textCascades'][idx]
        if value['status'] == 'unresolved':
            reason = value['reason']
            assert reason['kind'] in {'chartText','retainedContent','fieldParagraph'}
            n = nodes[reason['origin']['sourceOrdinal']]
            assert reason['origin'] == origin(n)
            continue
        text = value['text']
        assert text['chartPart'] == plan['chartPart'] and text['chartSha256'] == plan['chartSha256']
        assert text['bodySourceOrdinal'] == roots[0] and text['propertyRoots'] == roots[1:]
        if idx in unique: continue
        unique.add(idx)
        native_paragraphs = nodes[roots[0]].findall(A+'p')
        inherited = [nodes[r].find(A+'p/'+A+'pPr') for r in roots[1:]]
        assert len(text['paragraphs']) == len(native_paragraphs)
        for computed, p in zip(text['paragraphs'], native_paragraphs, strict=True):
            paragraphs += 1
            assert computed['sourceOrdinal'] == ids[p]
            chain = [n for n in [p.find(A+'pPr'), *inherited] if n is not None]
            attrs, origins = properties(chain, PARA, p)
            assert computed['attributes'] == attrs and computed['origins'] == origins
            assert computed['declarations'] == declarations(chain, PSLOTS)
            defaults = [n.find(A+'defRPr') for n in chain]
            native_runs = [r for r in p if r.tag in {A+'r', A+'br', A+'fld'}]
            assert len(computed['runs']) == len(native_runs)
            for i,(run,r) in enumerate(zip(computed['runs'],native_runs,strict=True)):
                assert run['sourceOrdinal'] == ids[r] and run['run'] == i
                assert run['kind'] == {A+'r':'text', A+'br':'break', A+'fld':'field'}[r.tag]
                character(run['style'], [r.find(A+'rPr'), *defaults]); styles += 1; runs += 1
            character(computed['endStyle'], [p.find(A+'endParaRPr'), *defaults]); styles += 1
    assert used == set(range(len(plan['textCascades'])))
    return dict(cascades=len(unique), paragraphs=paragraphs, runs=runs, styles=styles)

def main():
    cli, cases_file, output = sys.argv[1:]
    records=[]; rejected=0
    for case in json.loads(Path(cases_file).read_text()):
        source=Path(case['source']).read_bytes(); request=json.loads(Path(case['request']).read_text())
        response=json.loads(subprocess.check_output([cli,'pptx-chart-labels',case['request'],case['source']]))
        assert response['status'] == case['expectedStatus']
        record=dict(name=case['name'],status=response['status'])
        if response['status']=='planned':
            record.update(check(source,request,response))
            text = next((c for c in response['labels']['textCascades'] if c['status']=='cascaded'), None)
            if text:
                bad=copy.deepcopy(response)
                c=next(c for c in bad['labels']['textCascades'] if c['status']=='cascaded')
                c['text']['paragraphs'][0]['endStyle']['attributes']['size']=9999
                try: check(source,request,bad)
                except AssertionError: rejected+=1
                else: raise AssertionError('altered chart text size admitted')
        records.append(record)
    report=dict(profile='chart-text-independent-xml/1',cases=records,rejectedMutations=rejected,fontSelectionProven=False,shapingProven=False,renderingProven=False,officeWpsProven=False)
    with Path(output).open('x') as f:json.dump(report,f,indent=2);f.write('\n')
    print(json.dumps(dict(cases=len(records),rejectedMutations=rejected)))

if __name__=='__main__': main()
