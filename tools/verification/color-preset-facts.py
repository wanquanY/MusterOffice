"""Extract numerical facts from the explicit, digest-bound ECMA Part 1 PDF."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import fitz


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('pdf', type=Path); parser.add_argument('output', type=Path)
    args = parser.parse_args()
    digest = hashlib.sha256(args.pdf.read_bytes()).hexdigest()
    assert digest == '7cccfd0ad0e2ef89316acece15e44a8a088d0603ce4bba4e7b0361b50d13a37d', 'unexpected standard edition'
    with fitz.open(args.pdf) as document:
        text = '\n'.join(document[n].get_text() for n in range(2977,2994))
    rows = re.findall(r'\b([a-z][A-Za-z0-9]+) \([^)]*Preset\s+Color\)\s+Specifies a color with RGB value \((\d+),(\d+),(\d+)\)', text)
    facts = {name:list(map(int,(r,g,b))) for name,r,g,b in rows}
    assert len(rows)==len(facts)==190
    root = Path(__file__).resolve().parents[2]
    variants = re.findall(r'#\[serde\(rename = "(\w+)"\)\]\s+(\w+),', (root/'crates/mo-pptx/src/source/theme/names.rs').read_text().split('pub enum PresetColor {')[1])
    actual = {variant:list(map(int,(r,g,b))) for variant,r,g,b in re.findall(r'PresetColor::(\w+) => \[(\d+), (\d+), (\d+)\]', (root/'crates/mo-pptx/src/source/color/preset.rs').read_text())}
    assert set(facts)=={name for name,_ in variants}
    assert all(facts[name]==actual[variant] for name,variant in variants)
    args.output.write_text(json.dumps(facts,indent=2)+'\n')
    print(json.dumps({'standardPdfSha256':digest,'presets':len(facts),'factsSha256':hashlib.sha256(args.output.read_bytes()).hexdigest(),
                      'section':'20.1.10.50','pdfPages':[2978,2994], 'knownAliasDifference':{'long':facts['lightGoldenrodYellow'],'short':facts['ltGoldenrodYellow']}}))


if __name__ == '__main__': main()
