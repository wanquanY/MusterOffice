"""Measure owned miter-probe silhouettes from actual target-app canvas captures.

This reports image-space observations, not an invented screenshot acceptance gate.
Capture manifest crops must be grounded in the corresponding full UI snapshots.
"""
import hashlib
import json
import math
import plistlib
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path('.codex-work/miter-compat')


def read(path):
    return json.loads(Path(path).read_text())


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return {'path': str(path), 'byteLength': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def component(mask, x, y):
    """One owned V path per grid cell; seed on the interior of its left segment."""
    h, w = mask.shape
    near = np.argwhere(mask[y - 3:y + 4, x - 3:x + 4])
    assert len(near)
    py, px = near[0]
    seed = (y - 3 + int(py), x - 3 + int(px))
    pending = [seed]
    mask[seed] = False
    xmin = xmax = seed[1]
    ymin = ymax = seed[0]
    count = 0
    while pending:
        py, px = pending.pop()
        count += 1
        xmin, xmax = min(xmin, px), max(xmax, px)
        ymin, ymax = min(ymin, py), max(ymax, py)
        for ny, nx in [(py - 1, px), (py + 1, px), (py, px - 1), (py, px + 1)]:
            if 0 <= ny < h and 0 <= nx < w and mask[ny, nx]:
                mask[ny, nx] = False
                pending.append((ny, nx))
    return {'boundsInclusivePx': [xmin, ymin, xmax, ymax], 'pixels': count}


def silhouettes(image, mapping):
    a = np.asarray(image.convert('RGB'))
    # The source is an owned red shape on a white canvas, with no labels/images.
    mask = (a[:, :, 0] > 130) & (a[:, :, 1] < 100) & (a[:, :, 2] < 130)
    h, w = mask.shape
    out = []
    for item in mapping:
        sx = round((item['column'] * 200 + 100 - .55 * item['spread']) / 800 * w)
        sy = round((item['row'] * 200 + 88) / 600 * h)
        measured = component(mask, sx, sy)
        b = measured['boundsInclusivePx']
        measured['topPixelCenterInPageUnits'] = (b[1] + .5) / h * 600 - item['row'] * 200
        measured['bottomPixelCenterInPageUnits'] = (b[3] + .5) / h * 600 - item['row'] * 200
        out.append(measured)
    assert not mask.any(), 'unaccounted own red silhouette pixels'
    return out


inputs = read(ROOT / 'inputs.json')
captures = {c['name']: c for c in read(ROOT / 'wps-captures.json')}
validations = {c['name']: c for c in read(ROOT / 'independent.json')}
records = []
for c in inputs['files']:
    for k in ['page', 'pixels', 'export', 'pptx', 'plan']:
        assert entry(c[k]['path']) == c[k]
    validation = validations[c['name']]
    assert validation['result'] == 'passed' and validation['fileSha256'] == c['pptx']['sha256']
    capture = captures[c['name']]
    assert capture['pptxSha256'] == c['pptx']['sha256'], 'capture belongs to different input bytes'
    image = Image.open(capture['screenshotPath'])
    assert list(image.size) == capture['screenshotDimensions']
    canvas = image.crop(capture['canvasCrop'])
    canvas_path = ROOT / (c['name'] + '-wps-canvas.png')
    canvas.save(canvas_path)
    kernel = Image.frombytes('RGBA', (800, 600), Path(c['pixels']['path']).read_bytes())
    # Fixtures explicitly paint a white background: no premultiplication ambiguity.
    assert np.asarray(kernel)[:, :, 3].min() == 255
    kernel_path = ROOT / (c['name'] + '-kernel.png')
    kernel.save(kernel_path)
    authored = read(c['page']['path'])['page']['document']
    for item in c['mapping']:
        o = authored['objects'][item['object']]
        assert o['transform']['size'] == {'width': str(110 * 9525), 'height': str(140 * 9525)}
        assert o['transform']['origin'] == {'x': str((45 + item['column'] * 200) * 9525),
                                             'y': str((25 + item['row'] * 200) * 9525)}
        assert o['transform']['rotation'] == 0 and not o['transform']['flipHorizontal'] and not o['transform']['flipVertical']
        geometry = o['content']['geometry']
        assert geometry['viewport'] == {'width': str(100 * item['units']), 'height': str(100 * item['units'])}
        assert geometry['commands'] == [
            {'kind': kind, 'to': {'x': str(x * item['units']), 'y': str(y * item['units'])}}
            for kind, x, y in [('move', 50 - item['spread'], 80), ('line', 50, 10), ('line', 50 + item['spread'], 80)]
        ]
        stroke = o['appearance']['stroke']['value']
        assert stroke['width'] == str(item['width'] * 9525) and stroke['cap'] == 'flat'
        assert stroke['join'] == {'kind': 'miter', 'limit': item['limit']}
    native = silhouettes(kernel, c['mapping'])
    target = silhouettes(canvas, c['mapping'])
    observations = []
    for item, n, t in zip(c['mapping'], native, target, strict=True):
        run = item['spread'] * 1.1
        ratio = math.hypot(run, 98) / run
        radius = item['width'] / 2
        full_tip = 39 - radius * ratio
        bevel_tip = 39 - radius / ratio
        observations.append({**item, 'conventionalFullMiterRatio': ratio,
                             'analyticFullTipY': full_tip, 'analyticBevelTipY': bevel_tip,
                             'skiaMiterToBevelExpectedTipY': full_tip if item['limit'] / 100000 >= ratio else bevel_tip,
                             'kernel': n, 'wps': t,
                             'observedTopDifferenceInPageUnits': t['topPixelCenterInPageUnits'] - n['topPixelCenterInPageUnits']})
    records.append({'name': c['name'], 'pptx': c['pptx'], 'mapping': observations,
                    'kernelPreview': entry(kernel_path), 'wpsCanvas': entry(canvas_path),
                    'captureCrop': capture['canvasCrop'], 'launchReportedActivationSuppressed': capture['foregroundPreserved']})

# Varying the path's integer unit leaves physical geometry and native pixels identical.
c = next(c for c in inputs['files'] if c['name'] == 'coordinates')
pixels = np.frombuffer(Path(c['pixels']['path']).read_bytes(), dtype=np.uint8).reshape(600, 800, 4)
lookup = {}
for m in c['mapping']:
    row, col = m['row'], m['column']
    tile = pixels[row * 200:(row + 1) * 200, col * 200:(col + 1) * 200]
    if m['limit'] in lookup:
        assert np.array_equal(tile, lookup[m['limit']])
    else:
        lookup[m['limit']] = tile.copy()
coordinates = next(c for c in records if c['name'] == 'coordinates')['mapping']
spreads = []
for limit in lookup:
    values = [m['wps']['topPixelCenterInPageUnits'] for m in coordinates if m['limit'] == limit]
    spreads.append({'limit': limit, 'maxObservedTopSpreadInPageUnits': max(values) - min(values)})
info = plistlib.loads(Path('/Applications/wpsoffice.app/Contents/Info.plist').read_bytes())
report = {
    'format': 'musteroffice.miter-compat-observations/1',
    'application': {'name': 'WPS Office', 'version': info.get('CFBundleShortVersionString'), 'build': info.get('CFBundleVersion')},
    'inputs': entry(ROOT / 'inputs.json'), 'independentValidation': entry(ROOT / 'independent.json'),
    'silhouettePredicate': 'R > 130 and G < 100 and B < 130 in an owned red-on-white slide',
    'coordinateUnitStudy': {'nativePixelsExactlyInvariant': True, 'wpsTopSpreads': spreads},
    'cases': records,
    'scope': 'Actual background-opened WPS canvases. Pixel-center bounding observations include screenshot resampling, antialiasing and subpixel placement; they are not exact vector boundaries or a visual acceptance tolerance. No PowerPoint or edit-roundtrip result.',
}
(ROOT / 'observations.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'pages': len(records), 'objects': sum(len(c['mapping']) for c in records),
                  'nativeCoordinateUnitsInvariant': True, 'wpsCoordinateUnitTopSpreads': spreads}))
