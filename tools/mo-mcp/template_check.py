"""Real template queries/instances over the shared SDK-backed MCP computation.

Owned source and explicit synthetic font only. No Office/WPS or product claim.
"""
import copy
import io
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET
import zipfile

from compute_support import Files, Session, create, export, read, sha, write


def invocation(action, snapshot, name):
    return dict(request=dict(contractVersion='musteroffice.computation/1-draft',
                             requestId=name, profileId='presentations-author-model-v01-draft',
                             action=action), snapshot=snapshot)


def definition(snapshot, object_id, paragraph, run):
    return dict(format='musteroffice.presentation-template/1-draft',
                source=dict(documentId=snapshot['document']['id'], revision=snapshot['revision'],
                            semanticDigest=snapshot['semanticDigest']),
                parameters={'title': dict(label='Title', required=True, target=dict(
                    kind='textRun', object=object_id, paragraph=paragraph, run=run,
                    minScalars=1, maxScalars=100))})


def instantiate(source, description, name, text):
    return invocation(dict(kind='instantiateTemplate', documentId=name,
                           definition=description['definition'], templateDigest=description['templateDigest'],
                           bindings={'title': dict(kind='text', value=text)}), source, name)


def parts(data):
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        assert not package.testzip()
        result = {p: package.read(p) for p in package.namelist()}
    for name, body in result.items():
        if name.endswith(('.xml', '.rels')):
            ET.fromstring(body)
    return result


def package(files, name):
    folder = files.outputs / name
    index = read(folder / 'files.json')
    for asset in index:
        data = (folder / asset['file']).read_bytes()
        assert sha(data) == asset['asset']['sha256']
        assert len(data) == int(asset['asset']['byteLength'])
    pptx = next(a for a in index if a['asset']['role'] == 'pptx')
    previews = [a['asset']['sha256'] for a in index if a['asset']['role'] == 'preview']
    assert len(previews) == 2
    return (folder / pptx['file']).read_bytes(), previews


root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=False)
binary, worker = map(Path, sys.argv[2:4])
fixture = read('fixtures/presentations/delivery/input.json')
observations, parity = [], []
for era in ('2025-11-25', '2026-07-28'):
    files = Files(root / era, worker)
    cases = []
    with Session(binary, files, era, 'protocol') as client:
        caps = client.tool('mo_capabilities', {})
        assert {'describeTemplate', 'instantiateTemplate'} <= set(caps['operations'])
        assert not caps['businessJobs'] and not caps['productCommitted']

        def compute(name, value, inputs=None):
            args = files.invocation(name, value, inputs)
            result = client.computed(args)
            if value['request']['action']['kind'] not in ('import', 'export'):
                cases.append(dict(name=name, invocation=value, result=result))
                assert read(files.outputs / name / 'files.json') == []
            return result['result']

        def failed(name, value, code):
            args = files.invocation(name, value)
            result = client.tool('mo_presentations_compute', args)
            assert result['outcome'] == 'failed' and result['error']['code'] == code, result
            assert not (files.outputs / name).exists()
            files.clean()
            cases.append(dict(name=name, invocation=value, failure=result['error']))

        source = compute('source', create(fixture['document']))['snapshot']
        original = copy.deepcopy(source)
        protected = (files.outputs / 'source/result.json').read_bytes()
        title = source['document']['objects']['title:1']['content']['text']['paragraphs'][0]
        spec = definition(source, 'title:1', title['id'], title['runs'][0]['id'])
        query = invocation(dict(kind='describeTemplate', definition=spec), source, 'describe')
        description = compute('description', query)['description']
        assert description['examples'] == {'title': dict(kind='text', value='A A')}

        instances, packages, previews = [], [], []
        for name, text in [('instance-one', 'A A A'), ('instance-two', 'A')]:
            request = instantiate(source, description, name, text)
            result = compute(name, request)
            snapshot, receipt = result['snapshot'], result['receipt']
            assert snapshot['document']['id'] == name and receipt['transaction'] is None
            assert receipt['template']['source'] == spec['source']
            assert receipt['template']['scopeMap'] == dict(sourceDocument=source['document']['id'],
                                                           instanceDocument=name, localIdPolicy='preserve')
            assert snapshot['document']['objects']['title:1']['content']['text']['paragraphs'][0]['runs'][0]['content']['text'] == text
            value, inputs = export(files, snapshot, caps['exportRenderer'])
            compute('export-' + name, value, inputs)
            body, images = package(files, 'export-' + name)
            packages.append(body); previews.append(images); instances.append(snapshot)
            assert text.encode() in parts(body)['ppt/slides/slide1.xml']
        assert instances[0]['revision'] != instances[1]['revision'] and previews[0] != previews[1]
        edit = invocation(dict(kind='apply', documentId=instances[0]['document']['id'],
                               baseRevision=instances[0]['revision'], operations=[dict(operationId='title',
                               operation=dict(kind='setTitle', title='Independent instance'))]), instances[0], 'edit-instance')
        edited = compute('edited-instance', edit)
        assert edited['snapshot']['document']['title'] == 'Independent instance'
        assert 'template' not in edited['receipt'] and edited['receipt']['transaction']

        bad = instantiate(source, description, 'bad-digest', 'A')
        bad['request']['action']['templateDigest'] = '0' * 64
        failed('bad-digest', bad, 'REVISION_CONFLICT')
        bad = copy.deepcopy(query); bad['snapshot']['revision'] = '0' * 64
        failed('stale-source', bad, 'REVISION_CONFLICT')
        bad = instantiate(source, description, 'bad-type', 'A')
        bad['request']['action']['bindings']['title'] = dict(kind='color', value=dict(red=0, green=0, blue=0, alpha=255))
        failed('bad-type', bad, 'INPUT_INVALID')
        bad = copy.deepcopy(query); del bad['snapshot']
        failed('missing-source', bad, 'NOT_FOUND')
        bad = copy.deepcopy(query)
        bad['request']['action']['definition']['parameters']['title']['target']['object'] = 'nonexistent'
        failed('missing-target', bad, 'INPUT_INVALID')

        # Imported OOXML remains a native package, with its original identity
        # namespace. The template forks its document scope, not source part IDs.
        (files.inputs / 'native.pptx').write_bytes(packages[0])
        native_input = dict(file='native.pptx', info=dict(id='source', descriptor=dict(
            sha256=sha(packages[0]), byteLength=str(len(packages[0])),
            mediaType='application/vnd.openxmlformats-officedocument.presentationml.presentation'), verification='bytesSha256'))
        native = compute('native-source', dict(request=dict(contractVersion=caps['contractVersion'],
            requestId='native-source', profileId='presentations-author-model-v01-draft',
            action=dict(kind='import', documentId='native-source', source=dict(resourceId='source', assetId='source')))),
            [native_input])['snapshot']
        assert native['document']['title'] == source['document']['title']
        assert native['document']['sourceBindings']['profile'] == 'presentationml-retained-fields-v4-draft'
        accessibility = lambda document: sorted(
            (o['accessibility']['title'], o['accessibility']['description'], o['accessibility']['decorative'])
            for o in document['objects'].values())
        assert accessibility(native['document']) == accessibility(source['document'])
        matches = []
        for oid, obj in native['document']['objects'].items():
            for paragraph in obj['content'].get('paragraphs', []):
                for run in paragraph['runs']:
                    if run.get('text') == 'A A A':
                        matches.append((oid, paragraph['id'], run['id']))
        assert len(matches) == 1
        native_description = compute('native-description', invocation(dict(kind='describeTemplate',
            definition=definition(native, *matches[0])), native, 'native-description'))['description']
        instance = compute('native-instance', instantiate(native, native_description, 'native-instance', 'A A'))['snapshot']
        assert instance['document']['sourceBindings']['identityScope'] == 'native-source'
        value, inputs = export(files, instance, caps['exportRenderer'])
        value['request']['action']['settings']['resources'] = [dict(resourceId='source', assetId='source')]
        inputs = [native_input] + [v for v in inputs if v['info']['id'] == 'font:1']
        compute('native-export', value, inputs)
        body, native_previews = package(files, 'native-export')
        before, after = parts(packages[0]), parts(body)
        assert set(before) == set(after)
        changed = [p for p in before if before[p] != after[p]]
        assert changed == ['ppt/slides/slide1.xml'], changed
        assert b'A A</a:t>' in after['ppt/slides/slide1.xml']
        native_title = '原生模板 <&> title'
        title_edit = invocation(dict(kind='apply', documentId=instance['document']['id'],
            baseRevision=instance['revision'], operations=[dict(operationId='native-title',
                operation=dict(kind='setTitle', title=native_title))]), instance, 'native-title')
        titled = compute('native-title', title_edit)['snapshot']
        value, inputs = export(files, titled, caps['exportRenderer'])
        value['request']['action']['settings']['resources'] = [dict(resourceId='source', assetId='source')]
        inputs = [native_input] + [v for v in inputs if v['info']['id'] == 'font:1']
        compute('native-title-export', value, inputs)
        title_body, title_previews = package(files, 'native-title-export')
        title_parts = parts(title_body)
        title_changed = [p for p in after if after[p] != title_parts[p]]
        assert title_changed == ['docProps/core.xml'], title_changed
        assert ET.fromstring(title_parts['docProps/core.xml']).find(
            '{http://purl.org/dc/elements/1.1/}title').text == native_title
        assert title_previews == native_previews
        assert source == original and (files.outputs / 'source/result.json').read_bytes() == protected
        files.clean()
        observations.append(dict(protocol=era, calls=client.seq, inlineSuccesses=sum('result' in c for c in cases),
            failures=sum('failure' in c for c in cases), pptxParts=len(after), retainedParts=len(after)-len(changed),
            changedParts=changed, titleChangedParts=title_changed,
            exports=[sha(p) for p in [*packages, body, title_body]]))
        parity.append(cases)
        write(files.root / 'parity.json', cases)
assert parity[0] == parity[1]
assert observations[0]['exports'] == observations[1]['exports']
write(root / 'report.json', dict(format='musteroffice.template-mcp-check/1', status='passed',
    binarySha256=sha(binary.read_bytes()), workerSha256=sha(worker.read_bytes()), observations=observations,
    protocolOutputsIdentical=True, productCommitted=False, scope='Shared computation and owned native export only; not Office/WPS or product acceptance.'))
print(json.dumps(observations))
