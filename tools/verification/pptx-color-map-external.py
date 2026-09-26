"""Require exact output for owned color-map inheritance probes."""
import argparse
import json
from pathlib import Path
import fitz
from pptx_color_observer import observe


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('manifest', type=Path)
    parser.add_argument('--libreoffice', type=Path, required=True); parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args(); manifest = json.loads(args.manifest.read_text())
    version, cases = observe(manifest, args.libreoffice, args.output_dir)
    expected = {c['name']:c['externalColorProbe']['rgb'] for c in manifest['cases'] if 'externalColorProbe' in c}
    for case in cases:
        case['expectedRgb'] = expected[case['name']]
        case['passed'] = case['observedRgb'] == [tuple(case['expectedRgb'])]
    print(json.dumps({'format': 'musteroffice.color-map-external/1', 'application': version, 'pdfObserver': 'PyMuPDF ' + fitz.VersionBind,
                      'passed': sum(c['passed'] for c in cases), 'cases': cases,
                      'scope': 'Four owned solid-color samples opened/exported by LibreOffice. Not a kernel renderer, complete visual comparison or Office/WPS acceptance.'}, indent=2))
    if not all(c['passed'] for c in cases):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
