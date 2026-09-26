"""Create a product-reader fixture from a verified, owned native delivery.

Only MusterOffice outputs move into the product test corpus. Private product
source is neither read nor copied. This is fixture material, not publication.
"""
import hashlib,json,struct,sys,uuid
from pathlib import Path
source=Path('.codex-work/embedded-export/actual')
evidence=Path('docs/reviews/evidence/2026-09-26-embedded-export-verification.json')
assert hashlib.sha256(evidence.read_bytes()).hexdigest()=='35f680df59857e0dcda66db1dab36675715681a234cb0f6f43b2c8084bb664ae'
records={r['path']:r for r in json.loads(evidence.read_text())['artifacts']}
def read(name):
    path=source/name;raw=path.read_bytes();r=records[str(path)]
    assert len(raw)==r['byteLength'] and hashlib.sha256(raw).hexdigest()==r['sha256']
    return raw
root=Path(sys.argv[1]);root.mkdir(parents=True)
(root/'.gitignore').write_text('# Owned, generated test corpus. The repository ignores general user PPTX files.\n!*.pptx\n')
namespace=uuid.UUID('8152f9ab-02a7-4e31-92a0-a101fd302de8')
def ref(name,raw,media):
    return dict(content_id=str(uuid.uuid5(namespace,name)),byte_length=str(len(raw)),media_type=media,sha256=hashlib.sha256(raw).hexdigest())
bundle=json.loads(read('bundle.json'));inspection=json.loads(read('inspection.json'))
request=json.loads(read('request.json'));renderer=json.loads(read('renderer.json'))
domain=b'musteroffice.operation-request/1-draft'
payload=[request[k] for k in ['contractVersion','requestId','profileId','action']]
request_sha=hashlib.sha256(struct.pack('>Q',len(domain))+domain+json.dumps(payload,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
assets=[];files=[];names={}
for item in json.loads(read('files.json')):
    raw=read(item['file']);(root/item['file']).write_bytes(raw)
    asset=item['asset'];reference=ref(asset['id'],raw,asset['mediaType'])
    assert reference['sha256']==asset['sha256'] and reference['byte_length']==asset['byteLength']
    assets.append(dict(id=asset['id'],role=asset['role'],content=reference))
    files.append(dict(file=item['file'],content=reference));names[item['name']]=asset['id']
raw=read('bundle.json');bundle_ref=ref('public-bundle',raw,'application/vnd.musteroffice.bundle+json')
(root/'bundle.json').write_bytes(raw);files.append(dict(file='bundle.json',content=bundle_ref))
snapshot=json.loads(read('000.json'))
manifest=dict(version='presentation-artifact/4',artifact_id=str(uuid.uuid5(namespace,'artifact')),artifact_version='1',
    title=snapshot['document']['title'],
    kernel=dict(document_id=inspection['documentId'],revision=inspection['revision'],semantic_digest=inspection['semanticDigest'],
        request_sha256=request_sha,settings_sha256=inspection['settingsDigest'],
        renderer_profile=renderer['profile'],renderer_sha256=renderer['implementationSha256']),
    bundle=bundle_ref,assets=assets,model_asset_id=bundle['document']['modelAssetId'],
    pptx_asset_id=bundle['pptxAssetId'],quality_asset_id=names['quality'],
    pages=[dict(id=p['pageId'],image_asset_id=p['imageAssetId'],width=p['width'],height=p['height'],sample=p['sample']) for p in bundle['previews']],
    claims=bundle['claims'])
(root/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
(root/'files.json').write_text(json.dumps(files,indent=2)+'\n')
provenance=dict(source='MusterOffice owned delivery fixture; no private application source',evidenceSha256=hashlib.sha256(evidence.read_bytes()).hexdigest(),
    renderer=renderer,assets=len(assets),pages=len(manifest['pages']),scope='Actual PPTX and native pixels; only structure claim passed. Not full layout, editing, playback or Office/WPS acceptance.')
(root/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
print(json.dumps(dict(path=str(root),assets=len(assets),references=len(files))))
