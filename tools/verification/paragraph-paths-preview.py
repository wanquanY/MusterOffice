"""Diagnostic SVG from actual path scenes. Not the production paint backend."""
from pathlib import Path
from decimal import Decimal as D, localcontext
import hashlib,json
root=Path('.codex-work/paragraph-paths')
cases=['latin','bidi-brackets','devanagari','cjk-punctuation','emoji','variable-composite','cubic']
U=D(1<<32)
def n(raw):return str(D(raw)/U)
def xy(p):return n(p['x'])+' '+n(p['y'])
def commands(path):
    out=[]
    for c in path:
        k=c['kind']
        if k=='close':out.append('Z')
        elif k in ['move','line']:out.append(('M' if k=='move' else 'L')+xy(c['to']))
        elif k=='quadratic':out.append('Q'+xy(c['control'])+' '+xy(c['to']))
        else:out.append('C'+xy(c['control1'])+' '+xy(c['control2'])+' '+xy(c['to']))
    return ' '.join(out)
receipts=[]
with localcontext() as context:
    context.prec=60
    for name in cases:
        source=(root/(name+'.response.json')).read_bytes();scene=json.loads(source)['result']['scene'];b=scene['bounds'];assert b is not None
        xmin,ymin=D(b['min']['x'])/U,D(b['min']['y'])/U;xmax,ymax=D(b['max']['x'])/U,D(b['max']['y'])/U
        width=xmax-xmin;height=ymax-ymin;pad=max(width,height)/D(15)
        svg=[f'<svg xmlns="http://www.w3.org/2000/svg" width="900" height="900" viewBox="{xmin-pad} {ymin-pad} {width+2*pad} {height+2*pad}">',f'<rect x="{xmin-pad}" y="{ymin-pad}" width="{width+2*pad}" height="{height+2*pad}" fill="white"/>','<defs>']
        for i,p in enumerate(scene['paths']):svg.append(f'<path id="p{i}" d="{commands(p["commands"])}"/>')
        svg.append('</defs><g fill="#142c3b" fill-rule="nonzero">')
        for glyph in scene['glyphs']:svg.append(f'<use href="#p{glyph["path"]}" transform="translate({xy(glyph["origin"])})"/>')
        svg.append('</g></svg>\n');path=root/(name+'.svg');data='\n'.join(svg).encode();path.write_bytes(data)
        receipts.append({'name':name,'path':str(path),'byteLength':len(data),'sha256':hashlib.sha256(data).hexdigest(),'responseSha256':hashlib.sha256(source).hexdigest()})
(root/'previews.json').write_text(json.dumps({'format':'musteroffice.paragraph-paths-previews/1','scope':'Diagnostic unhinted monochrome SVG paths from core scene, no text nodes, fonts, scripts or external resources. The viewer supplies rasterization; this is not production rendering/Office comparison.','artifacts':receipts},indent=2)+'\n')
print(json.dumps({'previews':len(receipts)}))
