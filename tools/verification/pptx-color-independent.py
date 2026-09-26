"""Independent ZIP/MCE and 64-digit Decimal color oracle.

Reads actual file declarations and relationships, not Rust's source projection.
The selected profile is provisional; application observations are separate.
"""
import argparse
from decimal import Decimal, getcontext
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import posixpath
import zipfile
from lxml import etree as E
from mce_reference import project, A, P, R

NS = {'a': A, 'p': P}
getcontext().prec = 64
D = Decimal
ZERO, ONE = D(0), D(1)


class Unresolved(Exception):
    def __init__(self, kind, **data): self.reason = {'kind': kind, **data}


def unit(x): return min(ONE, max(ZERO, x))
def decode(x): return (abs(x)/D('12.92') if abs(x) <= D('.04045') else ((abs(x)+D('.055'))/D('1.055'))**D('2.4')).copy_sign(x)
def encode(x): return (D('12.92')*abs(x) if abs(x) <= D('.0031308') else D('1.055')*abs(x)**(ONE/D('2.4'))-D('.055')).copy_sign(x)
def local(node): return E.QName(node).localname
def sample(value, levels):
    # Remove only the oracle's 64-digit arithmetic tail at exact half-integer
    # boundaries. These owned inputs have at most 21 significant decimal digits;
    # 48 retained digits leave ample guard precision. RGB acceptance stays exact.
    scaled = (unit(value)*levels).quantize(D('1e-48'))
    return math.floor(scaled+D('.5'))

def rgb_to_hls(r, g, b):
    high, low = max(r,g,b), min(r,g,b); span = high-low; light = (high+low)/2
    if not span: return [ZERO,light,ZERO]
    sat = span/(high+low) if light <= D('.5') else span/(2-high-low)
    rc,gc,bc = [(high-v)/span for v in (r,g,b)]
    hue = bc-gc if r == high else 2+rc-bc if g == high else 4+gc-rc
    return [(hue/6+1)%1,light,sat]

def hls_to_rgb(hue, light, sat):
    if not sat: return [light]*3
    high = light*(1+sat) if light <= D('.5') else light+sat-light*sat
    low = 2*light-high
    def channel(h):
        h = (h+1)%1
        if h < ONE/6: return low+(high-low)*h*6
        if h < ONE/2: return high
        if h < D(2)/3: return low+(high-low)*(D(2)/3-h)*6
        return low
    return [channel(hue+ONE/3),channel(hue),channel(hue-ONE/3)]


def pct(raw):
    value = D(raw[:-1]) / 100 if raw.endswith('%') else D(raw) / 100000
    if not math.isfinite(float(value)): raise Unresolved('numericRange')
    return value


def environment(package, surface):
    parents = []; mapping = None; map_ref = None
    while surface:
        root, _, ordinals = project(package.read(surface.lstrip('/')), with_ordinals=True)
        node = root.find('p:clrMap', NS) if local(root) == 'sldMaster' else root.find('p:clrMapOvr/a:overrideClrMapping', NS)
        if mapping is None and node is not None:
            mapping = dict(node.attrib); map_ref = {'part': surface, 'sourceOrdinal': ordinals[node]}
        path = PurePosixPath(surface); relpath = str(path.parent / '_rels' / (path.name+'.rels')).lstrip('/')
        links = {}
        if relpath in package.namelist():
            for rel in E.fromstring(package.read(relpath)):
                if rel.get('TargetMode') != 'External':
                    links[rel.get('Type').removeprefix(R+'/')] = posixpath.normpath(posixpath.join(str(path.parent), rel.get('Target')))
        parents.append(links)
        surface = links.get('slideLayout' if local(root) == 'sld' else 'slideMaster') if local(root) != 'sldMaster' else None
        assert len(parents) <= 3
    scheme = None; scheme_ref = None; scheme_ord = None
    for links in reversed(parents):
        for name in ['theme','themeOverride']:
            if name not in links: continue
            root, _, ordinals = project(package.read(links[name].lstrip('/')), with_ordinals=True)
            colors = root.find('a:themeElements/a:clrScheme' if name == 'theme' else 'a:clrScheme', NS)
            if colors is not None:
                scheme = {local(slot):slot[0] for slot in colors if local(slot) != 'extLst'}
                scheme_ref = {'part':links[name], 'sourceOrdinal':ordinals[colors]}; scheme_ord = ordinals
    return mapping, map_ref, scheme, scheme_ref, scheme_ord


def verify(case, presets, coverage):
    raw = Path(case['source']).read_bytes(); assert hashlib.sha256(raw).hexdigest() == case['sourceSha256']
    request = json.loads(case['request']); actual = case['response']['palette']
    with zipfile.ZipFile(case['source']) as package: mapping, map_ref, scheme, scheme_ref, ordinals = environment(package, request['surface'])
    assert actual['colorMapping'] == map_ref and actual['colorScheme'] == scheme_ref
    assert actual['sourceSha256'] == request['expectedSourceSha256'] == case['sourceSha256']
    assert actual['surface'] == request['surface'] and actual['profile'] == request['profile']
    assert len(actual['colors']) == len(request['colors'])
    context = request['context']; outcomes = []; max16 = 0
    for name, result in zip(request['colors'], actual['colors']):
        deps = []; notices = []

        def resolve(name, stack):
            if name == 'phClr':
                if context['placeholder'] is None: raise Unresolved('missingPlaceholder')
                deps.append({'kind':'placeholder'})
                rgba = [D(v)/255 for v in context['placeholder']]
                return rgba[:3],rgba[3],None
            if name in ['dk1','lt1','dk2','lt2']: slot = name
            else:
                if mapping is None: raise Unresolved('missingColorMap')
                slot = mapping[name]
            if slot in stack: raise Unresolved('schemeCycle', slot=slot)
            if scheme is None: raise Unresolved('missingColorScheme')
            if slot not in scheme: raise Unresolved('missingThemeSlot', slot=slot)
            node = scheme[slot]; kind = local(node); coverage['models'].add(kind)
            deps.append({'kind':'theme', 'part':scheme_ref['part'], 'slot':slot, 'sourceOrdinal':ordinals[node]})
            alpha = ONE; hls = None
            if kind == 'srgbClr': rgb = [D(v)/255 for v in bytes.fromhex(node.get('val'))]
            elif kind == 'scrgbClr': rgb = [encode(pct(node.get(n))) for n in ['r','g','b']]
            elif kind == 'hslClr':
                hls = [D(node.get('hue'))/21600000, unit(pct(node.get('lum'))), unit(pct(node.get('sat')))]
                rgb = hls_to_rgb(*hls)
            elif kind == 'prstClr':
                preset = node.get('val'); coverage['presets'].add(preset); rgb = [D(v)/255 for v in presets[preset]]
                if preset == 'ltGoldenrodYellow' and 'presetAliasDiscrepancy' not in notices: notices.append('presetAliasDiscrepancy')
            elif kind == 'schemeClr':
                rgb, alpha, hls = resolve(node.get('val'), stack+[slot])
            else:
                assert kind == 'sysClr'; sys = node.get('val')
                if sys in context['systemColors']: rgb = context['systemColors'][sys]; origin = 'hostContext'
                elif node.get('lastClr') is not None: rgb = list(bytes.fromhex(node.get('lastClr'))); origin = 'fileLastColor'
                else: raise Unresolved('missingSystemColor', color=sys)
                deps.append({'kind':'system','color':sys,'origin':origin}); rgb = [D(v)/255 for v in rgb]
            for child in node:
                t = local(child); coverage['transforms'].add(t)
                value = (D(child.get('val'))/21600000 if t in ['hue','hueOff'] else pct(child.get('val'))) if child.get('val') is not None else None
                def apply(v): return unit(v+value if t.endswith('Off') else v*value if t.endswith('Mod') else value)
                if not t.startswith(('alpha','hue','lum','sat')) and t != 'comp': hls = None
                if t.startswith('alpha'): alpha = apply(alpha)
                elif t.startswith(('red','green','blue')):
                    i = 0 if t.startswith('red') else 1 if t.startswith('green') else 2
                    rgb[i] = apply(rgb[i])
                elif t.startswith(('hue','lum','sat')) or t == 'comp':
                    if hls is None: hls = rgb_to_hls(*map(unit,rgb))
                    i = 0 if t.startswith('hue') or t == 'comp' else 1 if t.startswith('lum') else 2
                    hls[i] = (hls[i]+D('.5'))%1 if t == 'comp' else apply(hls[i])
                    rgb = hls_to_rgb(*hls)
                elif t == 'tint': rgb = [encode(unit((1-value)+decode(v)*value)) for v in rgb]
                elif t == 'shade': rgb = [encode(unit(decode(v)*value)) for v in rgb]
                elif t == 'inv': rgb = [encode(unit(1-decode(v))) for v in rgb]
                elif t == 'gray':
                    rgb = [unit(sum(v*D(w) for v,w in zip(rgb,['.22','.72','.06'])))]*3
                    if 'grayWeightsProvisional' not in notices: notices.append('grayWeightsProvisional')
                elif t == 'gamma': rgb = [encode(unit(v)) for v in rgb]
                else:
                    assert t == 'invGamma'; rgb = [decode(unit(v)) for v in rgb]
            return rgb,alpha,hls
        try:
            rgb,alpha,_ = resolve(name, []); rgba = rgb+[alpha]
            outcome = {'status':'resolved','rgba8':[sample(v,255) for v in rgba],
                       'rgba16':[sample(v,65535) for v in rgba], 'clippedForSrgb':any(v < 0 or v > 1 for v in rgba[:3])}
            assert outcome['rgba8'] == result['outcome']['rgba8'], (case['name'], name, outcome, result)
            # At most one 16-bit step, declared before observing any comparison.
            delta = max(abs(a-b) for a,b in zip(outcome['rgba16'],result['outcome']['rgba16']))
            assert delta <= 1, (case['name'], name, delta); max16 = max(max16,delta)
            assert outcome['clippedForSrgb'] == result['outcome']['clippedForSrgb'], (case['name'], outcome, result)
        except Unresolved as error:
            outcome = {'status':'unresolved','reason':error.reason}
            assert outcome == result['outcome'], (case['name'], outcome, result)
        assert result['scheme'] == name and result['dependencies'] == deps and result['notices'] == notices, (case['name'], result, deps, notices)
        outcomes.append(outcome)
    return {'name':case['name'], 'sourceSha256':case['sourceSha256'], 'colors':len(outcomes), 'maxRgba16Difference':max16,
            'resolved':sum(o['status']=='resolved' for o in outcomes), 'unresolved':sum(o['status']=='unresolved' for o in outcomes)}


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('report',type=Path); parser.add_argument('presets',type=Path)
    parser.add_argument('--xsd-dir', type=Path)
    args = parser.parse_args(); report = json.loads(args.report.read_text()); presets = json.loads(args.presets.read_text())
    coverage = {'models':set(),'transforms':set(),'presets':set()}
    cases = [verify(c,presets,coverage) for c in report['cases'] if c['response']['status']=='evaluated']
    assert len(coverage['models'])==6 and len(coverage['transforms'])==28 and len(coverage['presets'])==190, coverage
    xsd_checked = []
    if args.xsd_dir:
        schema = E.XMLSchema(E.parse(str(args.xsd_dir/'dml-main.xsd'), E.XMLParser(resolve_entities=False,no_network=True)))
        for case in report['cases']:
            if case['name'].startswith('color-'):
                with zipfile.ZipFile(case['source']) as package: root, _ = project(package.read('ppt/theme/theme2.xml'))
                schema.assertValid(root)
                xsd_checked.append({'name':case['name'],'sourceSha256':case['sourceSha256'],'part':'/ppt/theme/theme2.xml'})
    print(json.dumps({'format':'musteroffice.color-independent/1','cases':cases, 'coverage':{k:sorted(v) for k,v in coverage.items()}, 'schemasChecked':xsd_checked,
                      'rgba8Tolerance':0,'rgba16Tolerance':1, 'scope':'Independent selected numerical profile and source bindings; not target application compatibility.'},indent=2))


if __name__ == '__main__': main()
