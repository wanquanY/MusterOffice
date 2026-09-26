"""Validate new wire samples against their actual Rust-generated schemas."""
import importlib.metadata
import json
from pathlib import Path
import jsonschema

ROOT = Path('.codex-work/text-page-runtime')
report = json.loads((ROOT/'parity.json').read_text())
request = json.loads(Path('contracts/generated/pptx-text-page-request.schema.json').read_text())
response = json.loads(Path('contracts/generated/pptx-text-page-raster-response.schema.json').read_text())
for schema in [request, response]:
    jsonschema.Draft202012Validator.check_schema(schema)
request_validator = jsonschema.Draft202012Validator(request)
response_validator = jsonschema.Draft202012Validator(response)
def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON member')
        result[key] = value
    return result
valid = invalid = responses = 0
for case in report['cases']:
    try:
        q = json.loads(Path(case['request']['path']).read_text(), object_pairs_hook=unique)
    except ValueError:
        assert not case['validRequest'], case['name']
        invalid += 1
    else:
        errors = list(request_validator.iter_errors(q))
        assert (not errors) == case['validRequest'], (case['name'], errors)
        valid += not errors
        invalid += bool(errors)
    response_validator.validate(json.loads(Path(case['response']['path']).read_text()))
    responses += 1
for key in ['failed', 'reused', 'rasterFailure']:
    response_validator.validate(report['fault'][key]); responses += 1
result = dict(format='musteroffice.text-page-runtime-contract-check/1',
              jsonschema=importlib.metadata.version('jsonschema'),
              validRequests=valid, rejectedRequests=invalid, responses=responses)
(ROOT/'contracts.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result))
