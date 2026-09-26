"""Independent exact stroke parameter and elementary pixel-geometry oracles.

No Skia stroker, Rust rounding routine, or recorded output serves as the oracle.
Curve pixels have parity coverage only; numeric bounds do not certify ink edges.
"""
import hashlib
import json
import struct
from fractions import Fraction as F
from pathlib import Path

U = 1 << 32

def exact(bits):
    return F(struct.unpack('<f', struct.pack('<I', bits))[0])

def nearest(value):
    if value == 0:
        return 0
    assert value > 0
    guess = struct.unpack('<I', struct.pack('<f', float(value)))[0]
    return min(range(max(0, guess-1), guess+2), key=lambda b: (abs(exact(b)-value), b & 1))

def verify_styles(draws, scale, words, offset, work):
    table, paints = [], []
    width_error = miter_error = F(0)
    for draw in draws:
        style = draw.get('stroke')
        if style is None:
            paints.append(0)
            continue
        width = F(int(style['width']) * scale['numerator'], scale['denominator'] * U)
        width_bits = nearest(width)
        width_error = max(width_error, abs(exact(width_bits) - width) * U)
        join = style['join']; miter = 0
        if join['kind'] in ['miter', 'miterClip']:
            limit = F(int(join['limit']), U); miter = nearest(limit)
            miter_error = max(miter_error, abs(exact(miter) - limit) * U)
        item = (width_bits, ['butt','round','square'].index(style['cap']),
                ['miter','round','bevel','miterClip'].index(join['kind']), miter)
        if item not in table:
            table.append(item)
        paints.append(table.index(item)+1)
    assert words[1] == 4 and words[8] == len(table)
    for item in table:
        assert words[offset:offset+4] == item
        offset += 4
    assert work['strokeStyles'] == len(table)
    assert work['strokeDraws'] == sum(bool(p) for p in paints)
    for field, error in [('strokeWidthErrorBound',width_error),('miterLimitErrorBound',miter_error)]:
        assert error <= int(work[field]) <= error + 2, (field, error, work[field])
    return offset, paints, {'styles':len(table), 'draws':sum(bool(p) for p in paints),
                            'exactWidthErrorRawQ32Pixels':str(width_error),
                            'exactMiterErrorRawQ32Ratio':str(miter_error)}

def main():
    root = Path('.codex-work/path-raster')
    report = json.loads((root/'parity.json').read_text())
    cases = {c['name']:c for c in report['cases']}
    records = []
    def pixel(name,x,y,color):
        c=cases[name]; b=Path(c['pixelsPath']).read_bytes()
        assert hashlib.sha256(b).hexdigest()==c['pixelsSha256']
        assert list(b[(y*64+x)*4:(y*64+x+1)*4])==color,(name,x,y,list(b[(y*64+x)*4:(y*64+x+1)*4]))
        records.append({'name':name,'pixel':[x,y],'expectedRgba':color,'pixelsSha256':c['pixelsSha256']})
    red=[255,0,0,255]; transparent=[0,0,0,0]
    for cap in ['butt','round','square']:
        name='stroke-cap-'+cap
        pixel(name,30,31,red)
        pixel(name,17,31,transparent if cap=='butt' else red)
        pixel(name,16,28,red if cap=='square' else transparent)
        pixel(name,30,20,transparent)
    for join in ['round','bevel','miter','miter-clipped','miter-zero']:
        name='stroke-join-'+join
        pixel(name,12,12,red if join=='miter' else transparent)
        pixel(name,14,15,red if join in ['round','miter'] else transparent)
        pixel(name,20,36,red)
    pixel('stroke-contour-open',12,32,transparent)
    pixel('stroke-contour-closed',12,32,red)
    pixel('stroke-fill-order-reset',20,20,[0,0,255,255])
    pixel('stroke-fill-order-reset',12,20,red)
    # A fill after a stroke must not inherit stroke paint state. Green over blue.
    pixel('stroke-fill-order-reset',40,40,[0,128,127,255])
    output={'format':'musteroffice.stroke-pixel-reference/1','samples':records,
            'scope':'Exact opaque/transparent interior pixel probes of elementary strokes and paint ordering. No curve-edge or Office/WPS fidelity claim.'}
    Path('.codex-work/stroke/pixel-reference.json').write_text(json.dumps(output,indent=2)+'\n')
    print(json.dumps({'exactPixelSamples':len(records)}))

if __name__ == '__main__':
    main()
