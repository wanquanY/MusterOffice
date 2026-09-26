"""Independent ZIP/XML/MCE typeface checks for the owned native-font corpus.

Verifies explicit draft precedence, not Office/WPS font classification or layout.
It selects declarations from XML and follows package relationships independently
of the Rust cascade/font results. Corpus styles are deliberately constrained.
"""
import hashlib
import json
from pathlib import Path
import posixpath
import zipfile
from lxml import etree as E
from mce_reference import project, A, P

ROOT = Path('.codex-work/text-fonts')
FIXTURES = ROOT / 'fixtures-final'
XSD = Path('.codex-work/ecma376/xsd')
NS = {'a': A, 'p': P}
REL = 'http://schemas.openxmlformats.org/package/2006/relationships'
MC = 'http://schemas.openxmlformats.org/markup-compatibility/2006'
SLIDE = '/ppt/slides/slide1.xml'
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
counts = dict(cases=0, named=0, unresolved=0, validParts=0, invalidParts=0,
              themeBindings=0, supplementalBindings=0, overrides=0)
cases, invalid, packages = [], [], set()
tags = dict(latin='latin', eastAsian='ea', complexScript='cs', symbol='sym')
slots = dict(lt='latin', ea='eastAsian', cs='complexScript')


def metadata(node):
    return dict(typeface=node.get('typeface'), panose=node.get('panose'),
                pitchFamily=int(v) if (v := node.get('pitchFamily')) is not None else None,
                charset=int(v) if (v := node.get('charset')) is not None else None)


for path in sorted(FIXTURES.glob('*.pptx')):
    data = path.read_bytes()
    digest = sha(data)
    assert path.stem.startswith(digest + '-')
    response_path = path.with_suffix('.json')
    response = json.loads(response_path.read_text())
    cascade = response['cascade']
    assert cascade['sourceSha256'] == digest
    packages.add(digest)
    with zipfile.ZipFile(path) as z:
        parts = {}

        def read(part):
            if part not in parts:
                parts[part] = project(z.read(part[1:]), True)
            return parts[part][0]

        def relation(part, suffix):
            name = posixpath.dirname(part) + '/_rels/' + posixpath.basename(part) + '.rels'
            if name[1:] not in z.namelist():
                return None
            root = E.fromstring(z.read(name[1:]), parser)
            found = [r for r in root if r.get('Type').endswith('/' + suffix)]
            assert len(found) <= 1
            if not found:
                return None
            assert found[0].get('TargetMode', 'Internal') == 'Internal'
            return posixpath.normpath(posixpath.join(posixpath.dirname(part), found[0].get('Target')))

        layout = relation(SLIDE, 'slideLayout')
        master = relation(layout, 'slideMaster')
        theme = relation(master, 'theme')
        selected = next((part for owner in [SLIDE, layout, master]
                         if (part := relation(owner, 'themeOverride')) is not None
                         and read(part).find('a:fontScheme', NS) is not None), theme)
        for part in [SLIDE, layout, master, theme, selected]:
            read(part)

        def shapes(part):
            return read(part).findall('p:cSld/p:spTree/p:sp', NS)

        def object_id(shape):
            return int(shape.find('p:nvSpPr/p:cNvPr', NS).get('id'))

        own = shapes(SLIDE)[0]
        assert cascade['object'] == dict(part=SLIDE, nativeId=object_id(own))
        ph = own.find('p:nvSpPr/p:nvPr/p:ph', NS)
        parent_shapes = {}
        if ph is not None:
            for part, key, value in [(layout, 'idx', ph.get('idx', '0')),
                                     (master, 'type', 'title' if ph.get('type') in ['title', 'ctrTitle'] else 'body')]:
                found = [s for s in shapes(part) if (p := s.find('p:nvSpPr/p:nvPr/p:ph', NS)) is not None
                         and p.get(key, '0' if key == 'idx' else 'obj') == value]
                assert len(found) == 1
                parent_shapes[part] = found[0]

        def at(part, node, shape=None, kind=None):
            origin = dict(sourceOrdinal=parts[part][2][node])
            if shape is not None:
                origin.update(kind='object', object=dict(part=part, nativeId=object_id(shape)))
            else:
                origin.update(kind='theme', part=part, defaultKind=kind)
            return dict(element=E.QName(node).localname, origin=origin)

        refs = []
        for part, shape in [(SLIDE, own), (layout, parent_shapes.get(layout))]:
            if shape is not None and (node := shape.find('p:style/a:fontRef', NS)) is not None:
                refs.append((node, at(part, node, shape)))
        for kind in ['txDef', 'lnDef', 'spDef']:
            node = read(theme).find(f'a:objectDefaults/a:{kind}/a:style/a:fontRef', NS)
            if node is not None:
                refs.append((node, at(theme, node, kind=kind)))
        if master in parent_shapes:
            shape = parent_shapes[master]
            if (node := shape.find('p:style/a:fontRef', NS)) is not None:
                refs.append((node, at(master, node, shape)))
        assert cascade.get('fontReference') == (refs[0][1] if refs else None)
        # These probes use direct run fonts or fontRef only. Other cascade
        # families are independently covered by text-cascade-reference.py.
        assert not any(r.findall('.//a:defRPr', NS) for r in [own, *parent_shapes.values()])
        defaults = read(theme).find('a:objectDefaults', NS)
        assert defaults is None or not defaults.findall('.//a:defRPr', NS)
        assert read(master).find('p:txStyles', NS) is None
        slot, script = response['slot'], response['script']
        node = own.find('p:txBody/a:p/a:r/a:rPr/a:' + tags[slot], NS)
        authored = metadata(node) if node is not None else None
        declared = at(SLIDE, node, own) if node is not None else (refs[0][1] if refs else None)
        actual_slot = cascade['paragraphs'][0]['runs'][0]['style']['declarations'].get(slot)
        assert actual_slot == (declared if authored is not None else None)
        reason, choice, font = None, None, None
        if authored is not None:
            name = authored['typeface']
            if not name:
                reason = dict(kind='emptyTypeface')
            elif name.startswith(('+mj-', '+mn-')):
                if name[4:] not in slots:
                    reason = dict(kind='unknownThemeToken', token=name)
                else:
                    choice = ('major' if name[1:3] == 'mj' else 'minor', slots[name[4:]])
            else:
                font = dict(typeface=name, declaredBy=declared, authoredFont=authored, theme=None, themeFont=None)
        elif not refs:
            reason = dict(kind='missingDeclaration')
        elif refs[0][0].get('idx') == 'none':
            reason = dict(kind='disabledThemeFont')
        elif slot == 'symbol':
            reason = dict(kind='symbolThemeFont')
        else:
            choice = (refs[0][0].get('idx'), slot)
        if choice:
            scheme = read(selected).find('a:fontScheme' if selected != theme else 'a:themeElements/a:fontScheme', NS)
            scheme_ref = dict(part=selected, sourceOrdinal=parts[selected][2][scheme])
            allowed = dict(fontScheme=['name'], majorFont=[], minorFont=[],
                           latin=['typeface', 'panose', 'pitchFamily', 'charset'],
                           ea=['typeface', 'panose', 'pitchFamily', 'charset'],
                           cs=['typeface', 'panose', 'pitchFamily', 'charset'], font=['script', 'typeface'])
            retained = [n for n in scheme.iter() if any(E.QName(a).namespace != MC
                        and (E.QName(a).namespace is not None or E.QName(a).localname not in allowed[E.QName(n).localname]) for a in n.attrib)]
            if retained:
                reason = dict(kind='retainedTheme', scheme=scheme_ref, sourceOrdinal=parts[selected][2][retained[0]])
            else:
                collection = scheme.find('a:' + choice[0] + 'Font', NS)
                member = collection.find('a:' + tags[choice[1]], NS)
                supplemental = None
                chosen = metadata(member) if member is not None and member.get('typeface') else None
                if chosen is None:
                    matches = [(i, n) for i, n in enumerate(collection.findall('a:font', NS)) if n.get('script') == script]
                    if script is None:
                        reason = dict(kind='scriptRequired')
                    elif not matches:
                        reason = dict(kind='missingSupplemental', script=script)
                    elif len(matches) > 1:
                        reason = dict(kind='ambiguousSupplemental', script=script)
                    else:
                        supplemental, member = matches[0]
                        chosen = metadata(member)
                if chosen is not None:
                    font = dict(typeface=chosen['typeface'], declaredBy=declared, authoredFont=authored,
                                theme=dict(scheme=scheme_ref, collection=choice[0], slot=choice[1], supplemental=supplemental),
                                themeFont=chosen)
                    counts['themeBindings'] += 1
                    counts['supplementalBindings'] += supplemental is not None
                    counts['overrides'] += selected != theme
        expected = dict(status='unresolved', reason=reason) if reason else dict(status='named', font=font)
        assert response['font'] == expected, (path, response['font'], expected)
        counts[expected['status']] += 1
        counts['cases'] += 1
        for part, (root, _, _) in parts.items():
            validator = dml if E.QName(root).namespace == A else pml
            valid = validator.validate(root)
            counts['validParts' if valid else 'invalidParts'] += 1
            if not valid:
                invalid.append(dict(sourceSha256=digest, part=part, reason=str(validator.error_log.last_error)))
    cases.append(dict(sourcePath=str(path), sourceSha256=digest, responsePath=str(response_path),
                      responseSha256=sha(response_path.read_bytes()), status=expected['status']))

assert counts['cases'] >= 24 and counts['named'] >= 17 and counts['overrides'] == 1
counts['uniquePackages'] = len(packages)
report = dict(format='musteroffice.native-font-library-reference/1', scope=__doc__,
              counts=counts, cases=cases, invalidParts=invalid)
(ROOT / 'font-reference.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(counts))
