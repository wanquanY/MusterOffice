"""Offline checks for design contracts only; does not validate a PPTX engine."""
from copy import deepcopy
from pathlib import Path
import json
import re

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parent
DOCS = ROOT.parent
NS = 'urn:musteroffice:design:0.4:'


def read(name):
    return json.loads((ROOT / name).read_text())


schemas = {p.name: json.loads(p.read_text()) for p in ROOT.glob('*.schema.json')}
registry = Registry().with_resources(
    (s['$id'], Resource.from_contents(s)) for s in schemas.values()
)
for spec in schemas.values():
    Draft202012Validator.check_schema(spec)


def manifest_errors(bundle):
    errors = []
    assets = {a['id']: a for a in bundle['assets']}
    if len(assets) != len(bundle['assets']):
        errors.append('duplicate asset ID')
    hashes = {}
    for asset in bundle['assets']:
        if int(asset['byteLength']) > 2**64 - 1:
            errors.append('byteLength exceeds u64')
        known = hashes.setdefault(asset['sha256'], asset['byteLength'])
        if known != asset['byteLength']:
            errors.append('same hash with different byteLength')

    def resolve(asset_id, role=None):
        asset = assets.get(asset_id)
        if asset is None:
            errors.append('dangling asset reference')
        elif role and asset['role'] != role:
            errors.append('asset role mismatch')
        return asset

    resolve(bundle['document']['modelAssetId'], 'editable-document')
    pptx = resolve(bundle['pptxAssetId'], 'pptx')
    if pptx and pptx['mediaType'] not in {
        'application/vnd.openxmlformats-officedocument.presentationml.presentation',
        'application/vnd.openxmlformats-officedocument.presentationml.template',
        'application/vnd.openxmlformats-officedocument.presentationml.slideshow',
        'application/vnd.ms-powerpoint.presentation.macroEnabled.12',
        'application/vnd.ms-powerpoint.template.macroEnabled.12',
        'application/vnd.ms-powerpoint.slideshow.macroEnabled.12',
    }:
        errors.append('presentation MIME mismatch')
    if 'playbackAssetId' in bundle:
        resolve(bundle['playbackAssetId'], 'playback-manifest')
    samples = set()
    for page in bundle['previews']:
        asset = resolve(page['imageAssetId'], 'preview')
        if asset and asset['mediaType'] != 'image/png':
            errors.append('preview MIME mismatch')
        key = (page['pageId'], json.dumps(page['sample'], sort_keys=True))
        if key in samples:
            errors.append('duplicate preview state')
        samples.add(key)
        if 'eventLogAssetId' in page['sample']:
            resolve(page['sample']['eventLogAssetId'])
    claim_keys = set()
    for claim in bundle['claims']:
        key = (claim['kind'], claim['profileId'])
        if key in claim_keys:
            errors.append('duplicate claim profile')
        claim_keys.add(key)
        if claim['kind'] != 'target-application' and claim['profileId'] != bundle['profileId']:
            errors.append('required claim uses another profile')
        if pptx and claim['subjectSha256'] != pptx['sha256']:
            errors.append('claim refers to other output bytes')
        for evidence_id in claim['evidenceAssetIds']:
            resolve(evidence_id, 'quality-report')
        if claim['kind'] == 'playback' and claim['status'] == 'passed':
            if 'playbackAssetId' not in bundle:
                errors.append('verified playback missing manifest')
    return errors


def errors_for(schema_name, value):
    validator = Draft202012Validator(schemas[schema_name], registry=registry)
    errors = [e.message for e in validator.iter_errors(value)]
    if errors:
        return errors
    if schema_name == 'delivery.schema.json':
        errors += manifest_errors(value)
    if schema_name == 'export-result.schema.json' and value['outcome'] == 'succeeded':
        bundle = value['bundle']
        errors += manifest_errors(bundle)
        if any(d['severity'] == 'error' for d in value['diagnostics']):
            errors.append('success with blocking diagnostic')
        if value['documentId'] != bundle['document']['documentId'] or value['revision'] != bundle['document']['revision']:
            errors.append('result document/revision mismatch')
        assets = {a['id']: a for a in bundle['assets']}
        for artifact in value['artifacts']:
            if assets.get(artifact['id']) != artifact:
                errors.append('result artifact differs from bundle')
        if bundle['pptxAssetId'] not in {a['id'] for a in value['artifacts']}:
            errors.append('delivered output missing from artifacts')
        for kind in ['structure','layout','native-editability']:
            claims = [c for c in bundle['claims'] if c['kind'] == kind]
            if not claims or any(c['status'] != 'passed' for c in claims):
                errors.append('success without required checks')
        if any(c['status'] not in ['passed','not_applicable'] for c in bundle['claims'] if c['kind'] == 'playback'):
            errors.append('success without playback disposition')
    if schema_name == 'capabilities.schema.json':
        features = value['features']
        ids = [f['id'] for f in features]
        if len(ids) != len(set(ids)):
            errors.append('duplicate capability ID')
        if {f['family'] for f in features} != {f'F{i:02}' for i in range(1,17)}:
            errors.append('missing family')
        if any(not f['id'].startswith(f['family']+'.') for f in features):
            errors.append('wrong capability family')
    return errors


positives = [
    ('capabilities.schema.json','capabilities.json'),
    ('export-request.schema.json','fixtures/export-request.json'),
    ('export-result.schema.json','fixtures/export-accepted.json'),
    ('export-result.schema.json','fixtures/export-succeeded.json'),
    ('export-result.schema.json','fixtures/export-failed.json'),
    ('delivery.schema.json','fixtures/static-delivery.json'),
]
for schema_name, fixture in positives:
    found = errors_for(schema_name, read(fixture))
    if found:
        raise AssertionError((fixture, found))

cases = []


def negative(name, schema_name, fixture, change):
    value = deepcopy(read(fixture))
    change(value)
    cases.append((name, schema_name, value))


request = 'fixtures/export-request.json'
accepted = 'fixtures/export-accepted.json'
delivery = 'fixtures/static-delivery.json'
succeeded = 'fixtures/export-succeeded.json'
negative('unknown input', 'export-request.schema.json', request, lambda v:v.update(tenantId='forged'))
negative('missing revision', 'export-request.schema.json', request, lambda v:v.pop('baseRevision'))
negative('wrong format', 'export-request.schema.json', request, lambda v:v['arguments'].update(format='pdf'))
negative('accepted as success', 'export-result.schema.json', accepted, lambda v:v.update(outcome='succeeded'))
negative('accepted terminal job', 'export-result.schema.json', accepted, lambda v:v['job'].update(state='succeeded'))
negative('accepted without polling', 'export-result.schema.json', accepted, lambda v:v['job'].pop('pollAfterMs'))
negative('target passed by self-check', 'delivery.schema.json', delivery, lambda v:v['claims'][-1].update(status='passed',basis='roundtrip',evidenceAssetIds=['quality-1']))
negative('target passed without evidence', 'delivery.schema.json', delivery, lambda v:v['claims'][-1].update(status='passed',basis='application-test'))
negative('wrong output hash', 'delivery.schema.json', delivery, lambda v:v['claims'][0].update(subjectSha256='a'*64))
negative('wrong verification profile', 'delivery.schema.json', delivery, lambda v:v['claims'][0].update(profileId='another-profile'))
negative('dangling preview', 'delivery.schema.json', delivery, lambda v:v['previews'][0].update(imageAssetId='missing'))
negative('duplicate resource', 'delivery.schema.json', delivery, lambda v:v['assets'].append(v['assets'][0]))
negative('unsafe byte length', 'delivery.schema.json', delivery, lambda v:v['assets'][0].update(byteLength='18446744073709551616'))
negative('byte length number', 'delivery.schema.json', delivery, lambda v:v['assets'][0].update(byteLength=4200))
negative('resource role wrong', 'delivery.schema.json', delivery, lambda v:v['assets'][1].update(role='image'))
negative('missing quality dimension', 'delivery.schema.json', delivery, lambda v:v['claims'].pop())
negative('event sample lacks history', 'delivery.schema.json', delivery, lambda v:v['previews'][0]['sample'].update(mode='event-frame'))
negative('dynamic without manifest', 'delivery.schema.json', delivery, lambda v:v['claims'][3].update(status='passed',basis='roundtrip',evidenceAssetIds=['quality-1']))
negative('result revision drift', 'export-result.schema.json', succeeded, lambda v:v.update(revision='wrong-revision'))
negative('artifact bytes drift', 'export-result.schema.json', succeeded, lambda v:v['artifacts'][0].update(sha256='f'*64))
negative('structural failure marked success', 'export-result.schema.json', succeeded, lambda v:v['bundle']['claims'][0].update(status='failed'))
negative('blocking diagnostic marked success', 'export-result.schema.json', succeeded, lambda v:v['diagnostics'].append({'code':'BROKEN','severity':'error','message':'Synthetic failure.','retryable':False}))
negative('missing NA reason', 'capabilities.schema.json','capabilities.json',lambda v:v['features'][0]['dimensions']['play'].pop('reason'))
negative('duplicate capability', 'capabilities.schema.json','capabilities.json',lambda v:v['features'].append(v['features'][0]))
for name, schema_name, value in cases:
    if not errors_for(schema_name, value):
        raise AssertionError('negative case accepted: '+name)

# Design plans must not claim unimplemented work has passed.
capabilities = read('capabilities.json')['features']
for feature in capabilities:
    assert feature['workflowStatus'] == 'not_started', feature['id']
    assert all(d['status'] in ['not_started','not_applicable'] for d in feature['dimensions'].values()), feature['id']
    for test_id in feature['requiredTests']:
        locations = [DOCS/'design/presentations/verification-and-roadmap.md', DOCS/'design/agent-interfaces.md']
        assert any(re.search(r'\| '+test_id+r'\b', p.read_text()) for p in locations), test_id
expected_counts = [6,10,9,5,9,8,4,6,6,6,4,6,6,6,7,6]
for index, expected in enumerate(expected_counts,1):
    family = f'F{index:02}'
    assert {f['id'] for f in capabilities if f['family']==family} == {f'{family}.{i:02}' for i in range(1,expected+1)}

print(json.dumps({'schemas':len(schemas),'positive_cases':len(positives),'negative_cases_rejected':len(cases),'capabilities':len(capabilities),'scope':'design contracts only; no engine or Office/WPS execution'},ensure_ascii=False,indent=2))
