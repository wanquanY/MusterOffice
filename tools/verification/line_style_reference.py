"""Independent tree/layer oracle for the owned native line style corpus.

Reads ZIP/XML directly; never uses SourceIndex or a kernel-provided parent graph.
Selects each property from ordered candidate columns rather than mutating a
partial Rust style. This is a draft-rule oracle, not application fidelity proof.
"""
import posixpath
import zipfile
from lxml import etree as E
from mce_reference import project, A, P
from line_reference import line, reference

NS = {'a': A, 'p': P}
DEFAULT = {'kind': 'profileDefault'}

def at(origin, ordinal):
    return {**origin, 'sourceOrdinal': ordinal}

def value(v, origin=DEFAULT):
    return {'value': v, 'declaredBy': origin}

def term(c, origin):
    return {k: c[k] for k in ['value', 'transforms']} | {'declaredBy': at(origin, c['sourceOrdinal'])}

class Unresolved(Exception):
    def __init__(self, reason): self.reason = reason

class Package:
    def __init__(self, path):
        with zipfile.ZipFile(path) as z:
            self.raw = {'/' + n: z.read(n) for n in z.namelist()}
        self.cache = {}

    def tree(self, part):
        if part not in self.cache:
            root, _, ordinals = project(self.raw[part], with_ordinals=True)
            self.cache[part] = root, ordinals
        return self.cache[part]

    def link(self, part, kind):
        folder, filename = posixpath.split(part)
        raw = self.raw.get(folder + '/_rels/' + filename + '.rels')
        if raw is None: return None
        targets = [n.get('Target') for n in E.fromstring(raw)
                   if n.get('Type').endswith('/' + kind) and n.get('TargetMode') != 'External']
        assert len(targets) <= 1
        return posixpath.normpath(posixpath.join(folder, targets[0])) if targets else None

    def objects(self, part):
        root, _ = self.tree(part)
        tree = root.find('p:cSld/p:spTree', NS)
        return {int(n.find('./*/p:cNvPr', NS).get('id')): n for n in tree.iter()
                if n.tag in {f'{{{P}}}{k}' for k in ['sp', 'pic', 'cxnSp', 'grpSp', 'graphicFrame']}
                and not any(p.tag == f'{{{P}}}extLst' for p in n.iterancestors())}

    def parent(self, part):
        name = E.QName(self.tree(part)[0]).localname
        return self.link(part, {'sld': 'slideLayout', 'sldLayout': 'slideMaster'}[name]) if name != 'sldMaster' else None

    def format(self, part):
        hierarchy = []
        while part:
            hierarchy.insert(0, part); part = self.parent(part)
        base = None; overrides = []
        for part in hierarchy:
            base = self.link(part, 'theme') or base
            over = self.link(part, 'themeOverride')
            if over: overrides.append(over)
        for target in reversed(([base] if base else []) + overrides):
            root, ordinals = self.tree(target)
            fmt = root.find('a:themeElements/a:fmtScheme', NS) if E.QName(root).localname == 'theme' else root.find('a:fmtScheme', NS)
            if fmt is not None: return target, fmt.find('a:lnStyleLst', NS), ordinals
        return None

    def layers(self, part, identity):
        selected = self.format(part); result = []; fallback = None; ref_seen = False
        while True:
            obj = self.objects(part)[identity]; _, ordinals = self.tree(part)
            binding = {'part': part, 'nativeId': identity}
            if E.QName(obj).localname not in ['sp', 'pic', 'cxnSp']:
                raise Unresolved({'kind': 'unsupportedObject', 'object': binding})
            origin = {'kind': 'object', 'object': binding}
            ref_node = obj.find('p:style/a:lnRef', NS)
            ref = reference(ref_node, ordinals) if ref_node is not None else None
            ph = None
            if ref:
                if ref['retainedOrdinals']:
                    raise Unresolved({'kind': 'retainedContent', 'origin': at(origin, ref['retainedOrdinals'][0])})
                if ref['color']: ph = term(ref['color'], origin)
                if not ref_seen: fallback = ph; ref_seen = True
            direct = obj.find('p:spPr/a:ln', NS)
            if direct is not None: result.append((line(direct, ordinals), origin, ph))
            if ref and ref['index']:
                if selected is None: raise Unresolved({'kind': 'missingFormatScheme', 'object': binding})
                theme, lines, nums = selected
                if ref['index'] > len(lines):
                    raise Unresolved({'kind': 'styleIndexOutOfRange', 'object': binding, 'index': ref['index'], 'available': len(lines)})
                origin = {'kind': 'theme', 'part': theme, 'via': binding,
                          'referenceOrdinal': ref['sourceOrdinal'], 'styleIndex': ref['index']}
                result.append((line(lines[ref['index'] - 1], nums), origin, ph))
            placeholder = obj.find('./*/p:nvPr/p:ph', NS)
            parent = self.parent(part)
            if placeholder is None or parent is None or placeholder.get('idx') == '4294967295': break
            # These owned fixtures deliberately have unique body placeholders.
            candidates = []
            for key, node in self.objects(parent).items():
                other = node.find('./*/p:nvPr/p:ph', NS)
                if other is None: continue
                kind = E.QName(self.tree(part)[0]).localname
                attr, default = ('idx', '0') if kind == 'sld' else ('type', 'obj')
                if placeholder.get(attr, default) == other.get(attr, default): candidates.append(key)
            assert len(candidates) == 1, (part, identity, candidates)
            part, identity = parent, candidates[0]
        return result, fallback

    def resolve(self, part, identity):
        try:
            layers, fallback = self.layers(part, identity)
            for declaration, origin, _ in layers:
                if declaration['retainedOrdinals']:
                    raise Unresolved({'kind': 'retainedContent', 'origin': at(origin, declaration['retainedOrdinals'][0])})
            def candidates(field):
                return [(d[field], at(o, d[field]['sourceOrdinal']), ph) for d, o, ph in layers if d[field] is not None]
            def first(field, default):
                return next((value(d[field], at(o, d['sourceOrdinal'])) for d, o, _ in layers if d[field] is not None), value(default))
            result = {k: first(k, v) for k, v in [('width', '9525'), ('cap', 'flat'), ('compound', 'sng'), ('alignment', 'ctr')]}
            for field, default in [('fill', 'none'), ('dash', 'preset'), ('join', 'round')]:
                options = candidates(field)
                selected, origin, _ = options[0] if options else ({'kind': default}, DEFAULT, None)
                kind = selected['kind']; output = {'kind': kind, 'declaredBy': origin}
                same = [(d, o, ph) for d, o, ph in options if d['kind'] == kind]
                if kind == 'retained':
                    raise Unresolved({'kind': 'unsupportedFill', 'origin': origin, 'nativeKind': selected['nativeKind']})
                if field == 'fill' and kind in ['gradient', 'pattern']:
                    # These are now typed source declarations. The full-line
                    # query still has no non-solid brush evaluation contract.
                    raise Unresolved({'kind': 'unsupportedFill', 'origin': origin, 'nativeKind': kind})
                if field == 'fill' and kind == 'solid':
                    colors = [(term(d['color'], o), ph) for d, o, ph in same if d['color'] is not None]
                    color, ph = colors[0] if colors else ({'value': {'kind': 'scheme', 'slot': 'bg1'}, 'transforms': [], 'declaredBy': DEFAULT}, None)
                    output['color'] = {'color': color, 'placeholder': ph if ph is not None else fallback}
                if field == 'dash':
                    if kind == 'custom': output['stops'] = selected['stops']
                    else: output['value'] = next((value(d['value'], o) for d, o, _ in same if d['value'] is not None), value('solid'))
                if field == 'join' and kind == 'miter':
                    output['limit'] = next((value(d['limit'], o) for d, o, _ in same if d['limit'] is not None), value('800000'))
                result[field] = output
            for field in ['head', 'tail']:
                options = candidates(field)
                result[field] = {'declaredBy': options[0][1] if options else DEFAULT}
                for key, default in [('kind', 'none'), ('width', 'med'), ('length', 'med')]:
                    result[field][key] = next((value(d[key], o) for d, o, _ in options if d[key] is not None), value(default))
            return {'status': 'resolved', 'line': result}
        except Unresolved as error:
            return {'status': 'unresolved', 'reason': error.reason}
