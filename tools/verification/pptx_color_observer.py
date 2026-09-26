"""Open owned color probes with LibreOffice and sample the exported PDF.

Development-only observer. No Office renderer or PDF library is linked into the
kernel. Requires explicitly supplied application, manifest and output directory.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as E
import zipfile
import fitz

P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
NS = {'p': P, 'a': A}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def observe(manifest, libreoffice, output_dir):
    selected = [c for c in manifest['cases'] if 'externalColorProbe' in c]
    assert selected, 'no explicit owned color probes'
    output_dir.mkdir(parents=True, exist_ok=True)
    for case in selected:
        assert digest(Path(case['path'])) == case['sha256']
        assert not (output_dir / (Path(case['path']).stem + '.pdf')).exists(), 'refusing to overwrite PDF'
    version = subprocess.check_output([str(libreoffice), '--version'], text=True).strip()
    with tempfile.TemporaryDirectory(prefix='profile-', dir=output_dir) as profile:
        result = subprocess.run([str(libreoffice), '-env:UserInstallation=' + Path(profile).resolve().as_uri(), '--headless',
                                 '--convert-to', 'pdf:impress_pdf_Export', '--outdir', str(output_dir.resolve()),
                                 *[str(Path(c['path']).resolve()) for c in selected]], text=True, capture_output=True, timeout=60)
        assert result.returncode == 0, result.stderr
    cases = []
    for case in selected:
        probe = case['externalColorProbe']; path = Path(case['path']); pdf = output_dir / (path.stem + '.pdf')
        assert pdf.is_file(), (case['name'], result.stdout, result.stderr)
        # Owned fixtures have fixed ordinary rectangular geometry on slide 1.
        # This oracle does not infer visual bounds of arbitrary user documents.
        with zipfile.ZipFile(path) as package:
            root = E.fromstring(package.read('ppt/slides/slide1.xml'))
        shape = next(s for s in root.findall('p:cSld/p:spTree/p:sp', NS) if s.find('p:nvSpPr/p:cNvPr', NS).get('name') == probe['object'])
        transform = shape.find('p:spPr/a:xfrm', NS); origin = transform.find('a:off', NS); size = transform.find('a:ext', NS)
        x = (int(origin.get('x')) + int(size.get('cx')) / 2) / 12700
        y = (int(origin.get('y')) + int(size.get('cy')) / 2) / 12700
        with fitz.open(pdf) as document:
            assert len(document) == 2
            pixels = document[probe['slide']].get_pixmap(matrix=fitz.Matrix(2, 2), clip=fitz.Rect(x-1, y-1, x+1, y+1), colorspace=fitz.csRGB, alpha=False)
            colors = sorted(set(tuple(pixels.samples[i:i+3]) for i in range(0, len(pixels.samples), 3)))
            cases.append({'name': case['name'], 'sourceSha256': case['sha256'], 'pdfSha256': digest(pdf), 'pages': len(document),
                          'sampleCenterPt': [x, y], 'samplePixels': pixels.width * pixels.height, 'observedRgb': colors})
    return version, cases
