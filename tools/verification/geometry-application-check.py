"""Validate owned application inputs and bind observed PDF bounds to query results.

Application differences are recorded, not treated as compatibility passes.
"""
import hashlib
import json
import zipfile
from pathlib import Path

from lxml import etree as E
from mce_reference import project

ROOT = Path('.codex-work/geometry-eval')
XSD = Path('.codex-work/ecma376/xsd')
schema = E.XMLSchema(E.parse(str(XSD / 'pml.xsd'), E.XMLParser(resolve_entities=False, no_network=True)))
observations = json.loads((ROOT / 'application-probes/observations.json').read_text())
queries = json.loads((ROOT / 'application-probes/parity.json').read_text())
expected = {'control': 72, 'sqrt-negative': 72, 'atan-quadrant': 8100000 / 12700}
cases = []


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


assert len(observations['cases']) == len(queries['cases']) == 9
for observation, query in zip(observations['cases'], queries['cases']):
    name = observation['name']
    assert name == query['name']
    raw = Path(observation['sourcePath']).read_bytes()
    assert sha(raw) == observation['sourceSha256'] == query['sourceSha256']
    assert sha(Path(observation['pdfPath']).read_bytes()) == observation['pdfSha256']
    response = Path(query['responsePath']).read_bytes()
    assert sha(response) == query['responseSha256']
    outcome = json.loads(response)['geometry']['objects'][0]['outcome']
    assert outcome['status'] == query['outcome']
    if name in expected:
        assert outcome['status'] == 'resolved'
        width = outcome['geometry']['paths'][0]['commands'][1]['to']['x'] / 12700
        assert width == query['evaluatedWidthPt']
        assert abs(width - expected[name]) < 1e-9, name
    else:
        assert outcome['status'] == 'unresolved'
        issue = 'arity' if name == 'extra-argument' else 'divisionByZero' if name == 'division-zero' else 'unknownReference'
        assert outcome['reason'].get('issue', outcome['reason']['kind']) == issue, name
    assert len(observation['redPaths']) == 1, name
    count = 0
    with zipfile.ZipFile(observation['sourcePath']) as package:
        for part in package.namelist():
            if part.endswith('.xml') and any(part.startswith(prefix) for prefix in (
                'ppt/slides/slide', 'ppt/slideLayouts/slideLayout', 'ppt/slideMasters/slideMaster'
            )):
                schema.assertValid(project(package.read(part))[0])
                count += 1
    assert count == 6, name
    cases.append({'name': name, 'sourceSha256': sha(raw), 'xsdParts': count,
                  'responseSha256': sha(response), 'pdfSha256': observation['pdfSha256']})

result = {'cases': cases, 'xsdParts': sum(case['xsdParts'] for case in cases),
          'xsdInputs': [{'path': str(path), 'sha256': sha(path.read_bytes())} for path in sorted(XSD.glob('*.xsd'))],
          'scope': 'Source XSD and explicit guide arithmetic or failure checks; PDF bounds are observations, not acceptance.'}
(ROOT / 'application-xsd.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'cases': len(cases), 'xsdParts': result['xsdParts']}))
