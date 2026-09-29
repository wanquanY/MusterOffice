"""Inspect capacity evidence from real maintained product outputs.

The independent ZIP/XML verifier still checks every generated instance. This
adds measurement coverage and exact comparison with a prior real run. It does
not turn capacity observations into visual or target-application acceptance.
"""
import argparse
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    'maintained', Path(__file__).with_name('product-template-stress-maintenance-evidence.py'))
maintained = importlib.util.module_from_spec(spec)
spec.loader.exec_module(maintained)


def products(prefix):
    defined = maintained.Stored(Path(str(prefix) + '-stress-definition'))
    stressed = maintained.Stored(Path(str(prefix) + '-stress-maintenance'))
    yield 'source', defined.product(defined.stage['source_render'])
    for case in stressed.stage['cases']:
        yield case['dimension'], stressed.product(case)


def fixed(value):
    assert isinstance(value, str) and str(int(value)) == value
    result = int(value)
    assert -(1 << 127) <= result < 1 << 127
    return result


def measurements(meta, files):
    assets = meta['delivery_assets']
    evidence = {}
    for entry in assets.values():
        asset = entry['asset']
        if asset['role'] != 'quality-report' or asset['mediaType'] != 'application/json':
            continue
        value = json.loads(files[entry['role']])
        assert value['format'] == 'musteroffice.preview-evidence/3-draft'
        assert value['pageId'] not in evidence
        evidence[value['pageId']] = asset, value
    bundle = meta['receipt']['bundle']
    assert set(evidence) == {preview['pageId'] for preview in bundle['previews']}
    pages = []
    for preview in bundle['previews']:
        asset, value = evidence[preview['pageId']]
        assert value['pptxSha256'] == assets[bundle['pptxAssetId']]['asset']['sha256']
        assert value['previewAsset'] == assets[preview['imageAssetId']]['asset']
        info = value['render']
        assert value['planSha256'] == info['page']['page']['sourceSha256']
        capacity = info['textCapacity']
        assert capacity['profile'] == 'drawingml-text-capacity-q32-v1-draft'
        assert len(capacity['frames']) == info['textFrames']
        visible = {(layer['part'], native_id) for layer in info['page']['page']['layers']
                   if layer['visible'] for native_id in layer['objects']}
        seen = set()
        for frame in capacity['frames']:
            key = frame['object']['part'], frame['object']['nativeId']
            assert key in visible and key not in seen
            seen.add(key)
            inner = frame['inner']
            height = fixed(inner['max']['y']) - fixed(inner['min']['y'])
            assert height > 0 and fixed(inner['max']['x']) > fixed(inner['min']['x'])
            content_height = fixed(frame['contentHeight'])
            assert content_height >= 0
            assert fixed(frame['verticalExcess']) == max(0, content_height - height)
            lines = frame['lineCount']
            for field in ['lineCount', 'horizontalOverflowLines', 'emergencyLines']:
                assert type(frame[field]) is int and 0 <= frame[field] <= lines
            assert (frame['firstHorizontalOverflow'] is None) == (frame['horizontalOverflowLines'] == 0)
            assert fixed(frame['maximumLeftExcess']) >= 0 and fixed(frame['maximumRightExcess']) >= 0
        pages.append(dict(pageId=preview['pageId'], evidenceSha256=asset['sha256'],
                          previewSha256=value['previewAsset']['sha256'], capacity=capacity))
    return pages


def claim_semantics(meta, files):
    # Evidence IDs are content-addressed and change with the pinned environment.
    # Require every claim to point to its own actual aggregate before comparing
    # all remaining fields; never discard an arbitrary differing claim field.
    aggregate = [entry for entry in meta['delivery_assets'].values()
                 if entry['asset']['mediaType'] == 'application/vnd.musteroffice.quality+json']
    assert len(aggregate) == 1
    entry = aggregate[0]
    quality = json.loads(files[entry['role']])
    for field in ['layoutQualityProven', 'nativeEditabilityProven', 'playbackProven', 'targetApplicationProven']:
        assert quality[field] is False
    result = []
    for claim in meta['receipt']['bundle']['claims']:
        assert claim['evidenceAssetIds'] == [entry['asset']['id']]
        result.append({key: value for key, value in claim.items() if key != 'evidenceAssetIds'})
    return result


def verify(prefix, prior_prefix):
    verified = maintained.verify(prefix)
    prior_verified = maintained.verify(prior_prefix)
    previous = dict(products(prior_prefix))
    results = []
    for name, (meta, files) in products(prefix):
        old_meta, old_files = previous.pop(name)
        # Renderer identity changes the environment and plan digest; it must not
        # alter the actual PPTX or PNG for this measurement-only implementation.
        bundle, old_bundle = meta['receipt']['bundle'], old_meta['receipt']['bundle']
        assert claim_semantics(meta, files) == claim_semantics(old_meta, old_files)
        ids = [bundle['pptxAssetId']] + [p['imageAssetId'] for p in bundle['previews']]
        old_ids = [old_bundle['pptxAssetId']] + [p['imageAssetId'] for p in old_bundle['previews']]
        assert len(ids) == len(old_ids)
        for current_id, old_id in zip(ids, old_ids):
            current = meta['delivery_assets'][current_id]
            old = old_meta['delivery_assets'][old_id]
            assert files[current['role']] == old_files[old['role']], (name, current_id)
        pages = measurements(meta, files)
        results.append(dict(dimension=name, pages=pages))
    assert not previous
    frames = [frame for result in results for page in result['pages'] for frame in page['capacity']['frames']]
    assert frames
    return dict(profile='product-template-text-capacity/1', qualityProven=False,
                sourceSha256=verified['sourceSha256'], priorSourceSha256=prior_verified['sourceSha256'],
                pptxAndPngBytesUnchanged=True, claimSemanticsUnchanged=True, computations=len(results),
                measuredPages=sum(len(r['pages']) for r in results), measuredFrames=len(frames),
                framesWithHorizontalOverflow=sum(f['horizontalOverflowLines'] > 0 for f in frames),
                framesWithVerticalOverflow=sum(fixed(f['verticalExcess']) > 0 for f in frames),
                results=results)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path, required=True)
    parser.add_argument('--prior-prefix', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    result = verify(args.prefix, args.prior_prefix)
    with args.report.open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'results'}))
