"""Independently verify actual files from the original maintained stress run.

Reuses the ZIP/XML inspector for all eight owned cases; consumes the original
content-store exports, not a synthesized SDK or simulated render result.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    'stress', Path(__file__).with_name('product-template-stress-evidence.py'))
stress = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stress)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()


class Stored:
    def __init__(self, root):
        self.stage = json.loads((root / 'stage.json').read_bytes())
        self.files = {}
        for item in json.loads((root / 'files.json').read_bytes()):
            assert Path(item['file']).name == item['file']
            ref = item['reference']
            data = (root / item['file']).read_bytes()
            assert len(data) == ref['byte_length'] and sha(data) == ref['sha256']
            assert ref['content_id'] not in self.files
            self.files[ref['content_id']] = ref, data

    def read(self, ref):
        expected, data = self.files[ref['content_id']]
        assert expected == ref
        return data

    def product(self, value):
        meta = json.loads(self.read(value['values']))
        assert meta['status'] == 'completed'
        files = {role: self.read(ref) for role, ref in value['files'].items()}
        assert set(files) == set(meta['files'])
        for role, data in files.items():
            declared = meta['files'][role]
            assert len(data) == declared['byte_length'] and sha(data) == declared['sha256']
            assert declared['media_type'] == value['files'][role]['media_type']
        for entry in meta.get('delivery_assets', {}).values():
            data, asset = files[entry['role']], entry['asset']
            assert sha(data) == asset['sha256'] and len(data) == int(asset['byteLength'])
            assert asset['mediaType'] == meta['files'][entry['role']]['media_type']
        if 'receipt' in meta:
            assets = meta['receipt']['bundle']['assets']
            assert len(assets) == len(meta['delivery_assets'])
            for asset in assets:
                assert meta['delivery_assets'][asset['id']]['asset'] == asset
        return meta, files


def verify(prefix):
    defined = Stored(Path(str(prefix) + '-stress-definition'))
    maintained = Stored(Path(str(prefix) + '-stress-maintenance'))
    prior, output = defined.stage, maintained.stage
    assert prior['stage'] == 'native_define' and output['stage'] == 'native_stress'
    assert output['validated_capability'] == 'finite-parameter-instance-computation'
    assert 'quality' not in output
    for key in ['version', 'decision', 'compilation_id', 'generation', 'source',
                'source_sha256', 'template', 'template_digest', 'compiler_profile',
                'font_profile_sha256', 'settings_digest', 'page_index', 'source_page_ids']:
        assert output[key] == prior[key], key
    assert prior['request_id'] != output['request_id']
    source_meta, source_files = defined.product(prior['source_render'])
    source_ids = json.loads(source_files['snapshot'])['document']['slideOrder']
    assert 1 <= len(source_ids) <= 40 and len(set(source_ids)) == len(source_ids)
    assert source_ids == output['source_page_ids']
    assert source_ids == [p['pageId'] for p in source_meta['receipt']['bundle']['previews']]
    plan = json.loads(maintained.read(output['plan']))
    assert [case['case_id'] for case in output['cases']] == list(range(output['case_count']))
    assert output['case_count'] == len(plan['cases']) == 8
    for case, planned in zip(output['cases'], plan['cases']):
        meta, files = maintained.product(case)
        assert json.loads(files['snapshot'])['document']['slideOrder'] == source_ids
        assert [p['pageId'] for p in meta['receipt']['bundle']['previews']] == source_ids
        bindings = plan['baseline'] | planned['bindings']
        for key in planned['omit']:
            del bindings[key]
        assert meta['binding_sha256'] == sha(canonical(bindings))
        assert case['dimension'] == planned['dimension']
        assert not planned['resources'], 'owned fixture has one text parameter'
        native_ref = lambda ref: ref | {'byte_length': str(ref['byte_length'])}
        parameters = dict(template_ref=native_ref(output['template']), template_digest=output['template_digest'],
                          settings_digest=output['settings_digest'], bindings=bindings, resources={},
                          source_resources={'template:source': native_ref(output['source'])},
                          document_id=f"stress:{output['plan']['sha256']}:{case['case_id']}")
        request = dict(operation='office_template_instantiate', parameters=parameters,
                       compiler_profile=output['compiler_profile'], font_profile_sha256=output['font_profile_sha256'])
        assert sha(canonical(request)) == case['compute_request_sha256']
        assert meta['instantiation']['documentId'] == parameters['document_id']

    def load(name):
        if name == 'stress-source':
            return defined.product(prior['source_render'])
        if name == 'stress-defined':
            return defined.product(prior['definition'])
        if name == 'stress-plan':
            return maintained.product({'values': output['plan_values'], 'files': {'stress_plan': output['plan']}})
        return maintained.product(output['cases'][int(name.removeprefix('stress-case-'))])

    result = stress.verify(prefix, loader=load)
    result['maintainedStressStageCompleted'] = True
    result['scope'] = 'Original maintained definition then all eight text cases; no typography or external application acceptance'
    result['stageBytes'] = len((Path(str(prefix) + '-stress-maintenance') / 'stage.json').read_bytes())
    result['storedContentCount'] = len(maintained.files)
    result['sourcePageIds'] = source_ids
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    result = verify(args.prefix)
    with args.report.open('x') as file:
        json.dump(result, file, indent=2)
        file.write('\n')
    print(json.dumps(dict(cases=len(result['cases']), maintainedStressStageCompleted=True, qualityProven=False)))
