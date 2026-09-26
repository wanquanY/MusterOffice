"""Independent XML cascade/provenance checks for the Rust library's owned probes.

Consumes actual PPTX/result pairs emitted by text_cascade.rs. This checks the
declared draft profile, not target-application behavior, fonts or rendering.
"""
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import project, A, P

ROOT = Path('.codex-work/text-cascade')
XSD = Path('.codex-work/ecma376/xsd')
NS = {'a': A, 'p': P}
SLIDE = '/ppt/slides/slide1.xml'
LAYOUT = '/ppt/slideLayouts/slideLayout2.xml'
MASTER = '/ppt/slideMasters/slideMaster2.xml'
THEME = '/ppt/theme/theme2.xml'
MAIN = '/ppt/presentation.xml'
sha = lambda b: hashlib.sha256(b).hexdigest()
archive = XSD.parent / 'OfficeOpenXML-XMLSchema-Transitional.zip'
assert sha(archive.read_bytes()) == 'd34187520749998af306faf1b730e568b0ca6d88ad24638a407c0a9bb4ca04fc'
with zipfile.ZipFile(archive) as z:
    for name in z.namelist():
        if name.endswith('.xsd'):
            assert z.read(name) == (XSD / name).read_bytes(), name
parser = E.XMLParser(resolve_entities=False, no_network=True)
pml = E.XMLSchema(E.parse(str(XSD / 'pml.xsd'), parser))
dml = E.XMLSchema(E.parse(str(XSD / 'dml-main.xsd'), parser))
boolean = lambda s: s in ['true', '1']
point = lambda s: {'kind': 'universalMeasure', 'value': s} if s[-1].isalpha() else {'kind': 'hundredthPoints', 'value': int(s)}
PARAGRAPH = {
    'leftMargin': ('marL', int, 0), 'rightMargin': ('marR', int, 0),
    'level': ('lvl', int, 0), 'indent': ('indent', int, 0), 'alignment': ('algn', str, 'l'),
    'defaultTabSize': ('defTabSz', str, '914400'), 'rightToLeft': ('rtl', boolean, False),
    'eastAsianLineBreak': ('eaLnBrk', boolean, True), 'fontAlignment': ('fontAlgn', str, 'base'),
    'latinLineBreak': ('latinLnBrk', boolean, False), 'hangingPunctuation': ('hangingPunct', boolean, True),
}
CHARACTER = {
    'kumimoji': ('kumimoji', boolean, False), 'language': ('lang', str, None),
    'alternativeLanguage': ('altLang', str, None), 'size': ('sz', int, 1800),
    'bold': ('b', boolean, False), 'italic': ('i', boolean, False), 'underline': ('u', str, 'none'),
    'strike': ('strike', str, 'noStrike'), 'kerning': ('kern', int, None), 'caps': ('cap', str, 'none'),
    'spacing': ('spc', point, {'kind': 'hundredthPoints', 'value': 0}), 'normalizeHeight': ('normalizeH', boolean, False),
    'baseline': ('baseline', str, '0'), 'noProof': ('noProof', boolean, False),
    'dirty': ('dirty', boolean, True), 'error': ('err', boolean, False),
    'smartClean': ('smtClean', boolean, True), 'smartId': ('smtId', int, 0), 'bookmark': ('bmk', str, None),
}
PSLOTS = {'lineSpacing': 'lnSpc', 'spaceBefore': 'spcBef', 'spaceAfter': 'spcAft',
          'bulletColor': 'buClr buClrTx', 'bulletSize': 'buSzTx buSzPts buSzPct',
          'bulletFont': 'buFont buFontTx', 'bullet': 'buNone buChar buAutoNum', 'tabs': 'tabLst'}
CSLOTS = {'line': 'ln', 'fill': 'noFill solidFill gradFill blipFill pattFill grpFill',
          'effects': 'effectLst effectDag', 'highlight': 'highlight', 'underlineLine': 'uLn uLnTx',
          'underlineFill': 'uFill uFillTx', 'latin': 'latin', 'eastAsian': 'ea', 'complexScript': 'cs',
          'symbol': 'sym', 'click': 'hlinkClick', 'mouseOver': 'hlinkMouseOver', 'rightToLeft': 'rtl'}
counts = dict(packages=0, cascaded=0, unresolved=0, paragraphs=0, characterStyles=0,
              attributeValues=0, attributeOrigins=0, declarations=0, validParts=0, invalidParts=0)
cases, invalid = [], []
for path in sorted((ROOT / 'fixtures').glob('*.pptx')):
    data = path.read_bytes()
    assert path.stem == sha(data)
    response_path = path.with_suffix('.json')
    response = json.loads(response_path.read_text())
    counts['packages'] += 1
    with zipfile.ZipFile(path) as z:
        parts = {name: project(z.read(name[1:]), True) for name in [SLIDE, LAYOUT, MASTER, THEME, MAIN]}
    for part, (root, _, _) in parts.items():
        validator = dml if part == THEME else pml
        valid = validator.validate(root)
        counts['validParts' if valid else 'invalidParts'] += 1
        if not valid:
            invalid.append(dict(source=path.stem, part=part, reason=str(validator.error_log.last_error)))
    cases.append(dict(sourcePath=str(path), sourceSha256=sha(data), responsePath=str(response_path), responseSha256=sha(response_path.read_bytes()), status=response['status']))
    if response['status'] == 'unresolved':
        counts['unresolved'] += 1
        continue
    counts['cascaded'] += 1
    result = response['text']
    assert result['sourceSha256'] == sha(data)
    assert result['profile'] == 'drawingml-text-cascade-draft-v1'
    def shapes(part):
        return parts[part][0].findall('p:cSld/p:spTree/p:sp', NS)
    def native_id(shape):
        return int(shape.find('p:nvSpPr/p:cNvPr', NS).get('id'))
    def origin(part, node, shape=None, default_kind=None):
        base = {'sourceOrdinal': parts[part][2][node]}
        if shape is not None:
            return dict(base, kind='object', object={'part': part, 'nativeId': native_id(shape)})
        if part == THEME:
            return dict(base, kind='theme', part=part, defaultKind=default_kind)
        return dict(base, kind='presentation' if part == MAIN else 'master', part=part)
    own = next(s for s in shapes(SLIDE) if native_id(s) == result['object']['nativeId'])
    ph = own.find('p:nvSpPr/p:nvPr/p:ph', NS)
    ph_type = ph.get('type', 'obj') if ph is not None else None
    master_type = 'title' if ph_type in ['title', 'ctrTitle'] else 'body'
    layout = master = None
    if ph is not None:
        candidates = [s for s in shapes(LAYOUT) if (q := s.find('p:nvSpPr/p:nvPr/p:ph', NS)) is not None and q.get('idx', '0') == ph.get('idx', '0')]
        assert len(candidates) == 1
        layout = candidates[0]
        candidates = [s for s in shapes(MASTER) if (q := s.find('p:nvSpPr/p:nvPr/p:ph', NS)) is not None and q.get('type', 'obj') == master_type]
        assert len(candidates) == 1
        master = candidates[0]
    def add(chain, node, part, shape=None, default_kind=None):
        if node is not None:
            chain.append((node, origin(part, node, shape, default_kind)))
    def values(actual, chain, mapping, local_p=None):
        expected_origins = {}
        for field, (attr, convert, default) in mapping.items():
            candidates = chain if field != 'level' else ([] if local_p is None else [local_p])
            found = next(((n, o) for n, o in candidates if n.get(attr) is not None), None)
            if found is not None:
                node, at = found
                expected, at = convert(node.get(attr)), at
            else:
                expected, at = default, {'kind': 'profileDefault'}
            assert actual['attributes'][field] == expected, (path.stem, field, actual['attributes'][field], expected)
            counts['attributeValues'] += 1
            if expected is not None:
                expected_origins[field] = at
        assert actual['origins'] == expected_origins, path.stem
        counts['attributeOrigins'] += len(expected_origins)
    def slots(actual, chain, mapping):
        expected = {}
        for slot, names in mapping.items():
            for node, at in chain:
                chosen = [c for c in node if E.QName(c).localname in names.split()]
                assert len(chosen) <= 1
                if chosen:
                    c = chosen[0]
                    part = at.get('part') or at['object']['part']
                    expected[slot] = {'element': E.QName(c).localname, 'origin': {**at, 'sourceOrdinal': parts[part][2][c]}}
                    break
        assert actual['declarations'] == expected, path.stem
        counts['declarations'] += len(expected)
    ps = own.findall('p:txBody/a:p', NS)
    assert len(ps) == len(result['paragraphs'])
    for p, actual in zip(ps, result['paragraphs']):
        counts['paragraphs'] += 1
        assert actual['sourceOrdinal'] == parts[SLIDE][2][p]
        ppr = p.find('a:pPr', NS)
        level = int(ppr.get('lvl', '0')) if ppr is not None else 0
        level_name = f'a:lvl{level + 1}pPr'
        chain = []
        add(chain, ppr, SLIDE, own)
        local_p = chain[0] if chain else None
        add(chain, own.find('p:txBody/a:lstStyle/' + level_name, NS), SLIDE, own)
        def template(shape, part):
            if shape is None:
                return
            paragraphs = [q for q in shape.findall('p:txBody/a:p', NS) if (a := q.find('a:pPr', NS)) is not None and int(a.get('lvl', '0')) == level]
            assert len(paragraphs) <= 1
            if paragraphs:
                add(chain, paragraphs[0].find('a:pPr', NS), part, shape)
            add(chain, shape.find('p:txBody/a:lstStyle/' + level_name, NS), part, shape)
        template(layout, LAYOUT)
        for kind in ['txDef', 'lnDef', 'spDef']:
            add(chain, parts[THEME][0].find(f'a:objectDefaults/a:{kind}/a:lstStyle/{level_name}', NS), THEME, default_kind=kind)
        template(master, MASTER)
        role = master_type if ph is not None else 'other'
        add(chain, parts[MASTER][0].find(f'p:txStyles/p:{role}Style/{level_name}', NS), MASTER)
        add(chain, parts[MAIN][0].find(f'p:defaultTextStyle/{level_name}', NS), MAIN)
        values(actual, chain, PARAGRAPH, local_p)
        slots(actual, chain, PSLOTS)
        characters = []
        for n, at in chain:
            if (c := n.find('a:defRPr', NS)) is not None:
                part = at.get('part') or at['object']['part']
                characters.append((c, {**at, 'sourceOrdinal': parts[part][2][c]}))
        runs = [r for r in p if E.QName(r).localname in ['r', 'br', 'fld']]
        assert len(runs) == len(actual['runs'])
        for position, (r, item) in enumerate(zip(runs, actual['runs'])):
            assert item['run'] == position and item['sourceOrdinal'] == parts[SLIDE][2][r]
            assert item['kind'] == {'r': 'text', 'br': 'break', 'fld': 'field'}[E.QName(r).localname]
            local = []
            add(local, r.find('a:rPr', NS), SLIDE, own)
            values(item['style'], local + characters, CHARACTER)
            slots(item['style'], local + characters, CSLOTS)
            counts['characterStyles'] += 1
        end = []
        add(end, p.find('a:endParaRPr', NS), SLIDE, own)
        values(actual['endStyle'], end + characters, CHARACTER)
        slots(actual['endStyle'], end + characters, CSLOTS)
        counts['characterStyles'] += 1
assert counts['packages'] > 20 and counts['paragraphs'] >= 180
report = dict(format='musteroffice.text-cascade-library-reference/1', scope=__doc__, counts=counts, cases=cases, invalidParts=invalid)
(ROOT / 'reference.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(counts))
