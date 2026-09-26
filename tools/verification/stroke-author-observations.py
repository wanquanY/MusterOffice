"""Validate owned PPTX files independently and observe actual LibreOffice strokes.

PDF observations report application differences; they are not kernel acceptance.
Run with the explicit development Python runtime (lxml, python-pptx, PyMuPDF).
"""
import argparse
import hashlib
import json
import math
import plistlib
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import fitz
import numpy as np
from PIL import Image

ROOT = Path('.codex-work/stroke-author/external')
parser = argparse.ArgumentParser()
parser.add_argument('--xsd-directory', type=Path, required=True)
parser.add_argument('--soffice', type=Path, required=True)
args = parser.parse_args()


def read(path):
    return json.loads(Path(path).read_text())


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return {'path': str(path), 'byteLength': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


inputs = read(ROOT / 'inputs.json')
independent = []
for case in inputs['cases']:
    for name in ['request', 'export', 'plan', 'pptx', 'pixels']:
        assert entry(case[name + 'Path'])['sha256'] == case[name + 'Sha256']
    run = subprocess.run([sys.executable, 'tools/verification/pptx-independent.py', case['pptxPath'],
                          '--request', case['exportPath'], '--xsd-directory', str(args.xsd_directory)],
                         capture_output=True, text=True, timeout=30)
    assert run.returncode == 0, (case['name'], run.stderr)
    result = json.loads(run.stdout)
    assert result['result'] == 'passed' and not result['differences']
    independent.append({'name': case['name'], **result})
(ROOT / 'independent.json').write_text(json.dumps(independent, indent=2) + '\n')

output = ROOT / 'libreoffice'
output.mkdir(exist_ok=True)
for case in inputs['cases']:
    (output / (case['name'] + '.pdf')).unlink(missing_ok=True)
with tempfile.TemporaryDirectory(prefix='mo-author-stroke-') as profile:
    run = subprocess.run([str(args.soffice), '-env:UserInstallation=' + Path(profile).as_uri(),
                          '--headless', '--convert-to', 'pdf:impress_pdf_Export', '--outdir', str(output),
                          *[c['pptxPath'] for c in inputs['cases']]],
                         capture_output=True, text=True, timeout=60)
    (ROOT / 'conversion.log').write_text(run.stdout + run.stderr)
    assert run.returncode == 0, run.stderr

records = []
for case in inputs['cases']:
    q = read(case['requestPath'])
    author = q['page']['document']['objects']['shape:1']['appearance']['stroke']['value']
    path = output / (case['name'] + '.pdf')
    with fitz.open(path) as pdf:
        assert len(pdf) == 1
        expected_size = [int(q['page']['document']['pageSize'][k]) / 12700 for k in ['width', 'height']]
        assert all(abs(a - b) < .02 for a, b in zip(pdf[0].rect[2:], expected_size))
        groups = []
        observed = []
        for d in pdf[0].get_drawings(extended=True):
            while groups and d['level'] <= groups[-1]['level']:
                groups.pop()
            if d['type'] == 'group':
                assert d['blendmode'] == 'Normal'
                groups.append(d)
            elif d['type'] in ['s', 'fs']:
                # This fixture has a single stroke in each transparency group.
                # Effective opacity includes ancestor groups (LO uses one for alpha).
                opacities = [g['opacity'] for g in groups]
                observed.append({'widthPt': d['width'], 'caps': list(d['lineCap']), 'join': d['lineJoin'],
                                 'paintOpacity': d['stroke_opacity'], 'groupOpacities': opacities,
                                 'opacity': d['stroke_opacity'] * math.prod(opacities),
                                 'color': list(d['color']), 'segments': len(d['items'])})
        content = b'\n'.join(pdf.xref_stream(x) for x in pdf[0].get_contents())
        # These single-object documents contain only a page fill and one stroke.
        # Record all explicit PDF M operators, without claiming inheritance proof.
        miter_values = [float(v) for v in re.findall(rb'([0-9]+(?:\.[0-9]+)?)\s+M\b', content)]
        ratio = q['viewport']['scale']['numerator'] / q['viewport']['scale']['denominator'] * 12700
        pdf[0].get_pixmap(matrix=fitz.Matrix(ratio, ratio)).save(ROOT / (case['name'] + '-libreoffice.png'))
    viewport = q['viewport']
    raw = np.frombuffer(Path(case['pixelsPath']).read_bytes(), dtype=np.uint8).reshape(viewport['height'], viewport['width'], 4).astype(np.uint16)
    # Production pixels are premultiplied. Composite directly over the white
    # target-app page; interpreting them as straight RGBA would darken edges.
    rgb = (raw[:, :, :3] + 255 - raw[:, :, 3:4]).astype(np.uint8)
    Image.fromarray(rgb).save(ROOT / (case['name'] + '-kernel.png'))
    checks = {}
    if author['kind'] == 'none':
        checks['noStroke'] = len(observed) == 0
    elif len(observed) == 1:
        d = observed[0]
        color = author['color']
        color = color['rgba'] if color['kind'] == 'srgb' else q['defaults']['themeColors'][color['slot']]
        expected_width = int(author['width']) / 12700
        checks = {
            'widthErrorPx96': abs(d['widthPt'] - expected_width) * 4 / 3,
            'widthWithinObservationTolerance': abs(d['widthPt'] - expected_width) * 4 / 3 <= .05,
            'capMatches': all(v == {'flat': 0, 'round': 1, 'square': 2}[author['cap']] for v in d['caps']),
            'joinMatches': d['join'] == {'miter': 0, 'round': 1, 'bevel': 2}[author['join']['kind']],
            'colorMatches': max(abs(a - color[k] / 255) for a, k in zip(d['color'], ['red', 'green', 'blue'])) <= 1e-5,
            'opacityWithinOneByte': abs(d['opacity'] - color['alpha'] / 255) <= 1 / 255,
        }
        if author['join']['kind'] == 'miter':
            expected = author['join']['limit'] / 100000
            checks['declaredMiterRatio'] = expected
            checks['lastExplicitPdfMiterRatioMatches'] = bool(miter_values) and abs(miter_values[-1] - expected) <= 1e-6
    records.append({'name': case['name'], 'pptx': entry(case['pptxPath']), 'pdf': entry(path),
                    'authorStroke': author, 'observedStrokes': observed, 'explicitPdfMiterRatios': miter_values,
                    'checks': checks,
                    'kernelPreview': entry(ROOT / (case['name'] + '-kernel.png')),
                    'libreOfficePreview': entry(ROOT / (case['name'] + '-libreoffice.png'))})

info = plistlib.loads((args.soffice.parent.parent / 'Info.plist').read_bytes())
report = {
    'format': 'musteroffice.stroke-author-external-observations/1',
    'application': {'name': 'LibreOffice', 'version': info.get('CFBundleShortVersionString'), 'build': info.get('CFBundleVersion')},
    'inputs': entry(ROOT / 'inputs.json'), 'independentFileValidation': entry(ROOT / 'independent.json'),
    'widthObservationTolerancePx96': .05, 'cases': records,
    'scope': 'Owned native editable PPTX XSD/structure checks and actual PDF stroke observations. '
             'Differences are retained, not redefined as passing. No full visual, edit-roundtrip, WPS, PowerPoint or replacement acceptance.',
}
(ROOT / 'observations.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'independentFilesPassed': len(independent), 'observedPages': len(records),
                  'miterDifferences': [r['name'] for r in records if r['checks'].get('lastExplicitPdfMiterRatioMatches') is False],
                  'missingSingleStroke': [r['name'] for r in records if r['authorStroke']['kind'] != 'none' and len(r['observedStrokes']) != 1]}))
