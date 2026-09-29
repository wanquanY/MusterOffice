"""Independently inspect every actual owned text stress case and stored PPTX.

Uses Python ZIP/XML, not the SDK parser. Synthetic code-point coverage does not
prove typography, native editing roundtrips, playback, or application fidelity.
"""
import argparse
import importlib.util
import io
import json
from pathlib import Path
import struct
import xml.etree.ElementTree as ET
import zipfile

spec = importlib.util.spec_from_file_location('instances', Path(__file__).with_name('product-template-instances-evidence.py'))
instances = importlib.util.module_from_spec(spec)
spec.loader.exec_module(instances)


def verify(prefix, loader=None):
    load = loader or (lambda name: instances.load(prefix, name))
    source, source_files = load('stress-source')
    defined, defined_files = load('stress-defined')
    planned, plan_files = load('stress-plan')
    plan_bytes = plan_files['stress_plan']
    assert instances.sha(plan_bytes) == planned['plan_sha256']
    plan = json.loads(plan_bytes)
    assert plan['version'] == 'presentation-native-stress-plan/1'
    assert plan['policy'] == 'presentation.native-template-stress@1'
    assert plan['template_digest'] == defined['description']['templateDigest']
    assert plan['baseline'] == defined['description']['examples']
    assert plan['settings_digest'] == source['environment']['settingsDigest']
    assert plan['font_sha256'] == source['environment']['fontSha256']
    assert 'content_id' not in plan_bytes.decode(), 'owned plan contains only byte resource identities'
    package = json.loads(defined_files['template_document'])
    target = package['definition']['parameters']['headline']['target']
    binding = package['snapshot']['document']['sourceBindings']['objects'][target['object']]
    run = binding['runs'][target['run']]
    part = binding['part'].lstrip('/')
    slide_parts = package['snapshot']['document']['sourceBindings']['slides']
    assert binding['part'] in slide_parts.values(), 'owned case targets a slide-local run'
    baseline = plan['baseline']['headline']['value']
    # Independent expected cases for this one required text target. Exact scalar
    # generation and required dimensions are checked, not read as pass flags.
    dimensions = [('source-values', baseline), ('text-minimum', 'A'),
                  ('text-maximum-latin-wide', 'W' * 16), ('text-maximum-latin-narrow', 'i' * 16),
                  ('text-maximum-cjk', '演示文稿' * 4), ('text-maximum-rtl', ('مرحبا' * 4)[:16]),
                  ('text-maximum-combining', 'A\u0301' * 8), ('text-maximum-word-breaks', 'A A ' * 4)]
    assert len(plan['cases']) == len(dimensions) == planned['case_count']
    original = instances.asset(source, source_files, source['receipt']['bundle']['pptxAssetId'])
    results = []
    for index, (case, (dimension, expected_text)) in enumerate(zip(plan['cases'], dimensions)):
        assert case['id'] == index and case['dimension'] == dimension
        assert not case['resources'] and not case['omit']
        assert case['bindings'] == ({} if index == 0 else {'headline': {'kind': 'text', 'value': expected_text}})
        result, files = load(f'stress-case-{index}')
        assert result['environment'] == source['environment']
        assert result['template_digest'] == plan['template_digest']
        receipt = result['receipt']
        assert result['instantiation']['template']['boundParameters'] == ['headline']
        for key in ('documentId', 'revision', 'semanticDigest'):
            assert receipt[key] == result['instantiation'][key]
        snapshot = json.loads(files['snapshot'])
        paragraphs = snapshot['document']['objects'][target['object']]['content']['paragraphs']
        paragraph = next(p for p in paragraphs if p['id'] == target['paragraph'])
        assert next(r for r in paragraph['runs'] if r['id'] == target['run'])['text'] == expected_text
        pptx = instances.asset(result, files, receipt['bundle']['pptxAssetId'])
        with zipfile.ZipFile(io.BytesIO(original)) as before, zipfile.ZipFile(io.BytesIO(pptx)) as after:
            assert before.namelist() == after.namelist()
            changed = [p for p in before.namelist() if before.read(p) != after.read(p)]
            assert changed == ([] if index == 0 else [part])
            for item in before.infolist():
                if item.filename not in changed:
                    assert instances.compressed(original, item) == instances.compressed(pptx, after.getinfo(item.filename))
            a, b = ET.fromstring(before.read(part)), ET.fromstring(after.read(part))
            text = instances.target_text(b, binding, run)
            assert text.text == expected_text
            text.text = instances.target_text(a, binding, run).text
            assert ET.tostring(a) == ET.tostring(b)
            preserved = len(before.namelist()) - len(changed)
        previews = []
        for page, (preview, base) in enumerate(zip(receipt['bundle']['previews'], source['receipt']['bundle']['previews'])):
            png = instances.asset(result, files, preview['imageAssetId'])
            assert png[:8] == b'\x89PNG\r\n\x1a\n'
            assert struct.unpack('>II', png[16:24]) == (640, 360)
            assert preview['pageId'] == base['pageId']
            if slide_parts[preview['pageId']] != binding['part']:
                assert png == instances.asset(source, source_files, base['imageAssetId'])
            previews.append(dict(sha256=instances.sha(png), byteLength=len(png)))
        assert len(previews) == 2
        claims = {c['kind']: c['status'] for c in receipt['bundle']['claims']}
        assert claims == {'structure': 'passed', 'layout': 'not_proven', 'native-editability': 'not_proven',
                          'playback': 'not_proven', 'target-application': 'not_proven'}
        results.append(dict(caseId=index, dimension=dimension, scalars=len(expected_text),
                            pptxSha256=instances.sha(pptx), pptxBytes=len(pptx), changedParts=changed,
                            preservedCompressedParts=preserved, actualAssets=len(result['delivery_assets']),
                            previews=previews, claims=claims))
    return dict(planSha256=instances.sha(plan_bytes), planBytes=len(plan_bytes), fontSha256=plan['font_sha256'],
                sourceSha256=instances.sha(original), cases=results, allRequiredTextCasesExecuted=True,
                maintainedStressStageCompleted=False, qualityProven=False,
                scope='Eight owned text cases via real SDK/worker; no multilingual typography acceptance')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    result = verify(args.prefix)
    with args.report.open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print(json.dumps(dict(planSha256=result['planSha256'], cases=len(result['cases']),
                         actualAssets=sum(c['actualAssets'] for c in result['cases']), qualityProven=False)))


if __name__ == '__main__':
    main()
