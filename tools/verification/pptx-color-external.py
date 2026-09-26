"""Record application observations, including differences; never call them passes."""
import argparse
import json
from pathlib import Path
import fitz
from pptx_color_observer import observe


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('manifest', type=Path); parser.add_argument('parity_report', type=Path)
    parser.add_argument('--libreoffice', type=Path, required=True); parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args(); manifest = json.loads(args.manifest.read_text()); report = json.loads(args.parity_report.read_text())
    version, cases = observe(manifest, args.libreoffice, args.output_dir)
    expected = {c['name']:c for c in report['cases']}
    for case in cases:
        reference = expected[case['name']]
        assert reference['sourceSha256'] == case['sourceSha256']
        palette = reference['response']['palette']; result = palette['colors'][0]['outcome']
        assert result['status']=='resolved' and result['rgba8'][3]==255
        case['kernelRgb'] = result['rgba8'][:3]; case['profile'] = palette['profile']
        case['equal'] = case['observedRgb'] == [tuple(case['kernelRgb'])]
        case['maxChannelDifference'] = max(abs(v-w) for rgb in case['observedRgb'] for v,w in zip(rgb,case['kernelRgb']))
    print(json.dumps({'format':'musteroffice.color-external/1','application':version,'pdfObserver':'PyMuPDF '+fitz.VersionBind,
                      'observed':len(cases),'equal':sum(c['equal'] for c in cases),'different':sum(not c['equal'] for c in cases),
                      'cases':cases,'scope':'Exact observations of owned solid-color probes. Differences remain unresolved; neither Office/WPS acceptance nor kernel rendering certification.'},indent=2))


if __name__ == '__main__': main()
