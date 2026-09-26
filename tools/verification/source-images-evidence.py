"""Seal completed source image checks without replacing historical evidence."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote

ROOT = Path('.codex-work/source-images')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-image-brush-completed-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-source-images-verification.json')
def entry(path):
    p = Path(path); data = p.read_bytes()
    return dict(path=str(p),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())

assert entry(PREVIOUS)['sha256'] == '4f407ae1696efeb49337e113f0a399576f33f44f5994bc621733883f0c86c31a'
old = json.loads(PREVIOUS.read_text())
allowed = {
    'Cargo.lock','README.md','contracts/README.md','crates/mo-kernel-api/src/lib.rs',
    'crates/mo-pptx/Cargo.toml','crates/mo-pptx/src/source.rs','crates/mo-pptx/src/source/fill/resolve/types.rs',
    'crates/mo-wasm/src/lib.rs','docs/README.md','docs/implementation/development.md',
    'docs/implementation/progress.md','tools/mo-cli/src/main.rs',
    'tools/mo-contract-codegen/src/main.rs','tools/verification/contracts.py',
}
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
assert changed == allowed, (changed-allowed,allowed-changed)
added = {
    'crates/mo-pptx/src/source/images.rs','crates/mo-pptx/src/source/images/types.rs',
    'crates/mo-pptx/tests/source_images.rs','crates/mo-pptx/tests/support/images.rs',
    'crates/mo-kernel-api/src/pptx_images.rs','tools/mo-cli/src/images.rs',
    'contracts/generated/pptx-image-query.schema.json','contracts/generated/pptx-image-response.schema.json',
    'packages/contracts/src/generated/pptx-image-query.ts','packages/contracts/src/generated/pptx-image-response.ts',
    'docs/implementation/source-images.md','tools/verification/source-images-parity.mjs',
    'tools/verification/source-images-reference.py','tools/verification/source-images-regressions.mjs',
    'tools/verification/source-images-evidence.py',
}
sources = {r['path'] for r in old['sourceFiles']} | added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p

logs = ['workspace-tests','image-tests','clippy','fmt','workers-build','schema-write','schema-check',
        'types-write','types-check','contracts','wasm-build','bindgen','parity','reference','regressions','recent-regressions']
for name in logs:
    text = (ROOT/(name+'.log')).read_text()
    assert not any(v in text for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
for name in ['clippy','workers-build','wasm-build']:
    assert 'Finished' in (ROOT/(name+'.log')).read_text(), name
test_log = (ROOT/'workspace-tests.log').read_text()
assert 'Doc-tests mo_xml' in test_log and test_log.rstrip().endswith('finished in 0.00s')
tests = re.findall(r'^test (.+) \.\.\. ok$',test_log,re.M)
new_tests = [n for n in tests if n not in old['rustTestNames']]
assert len(tests) == 515 and len(new_tests) == 10
assert len(re.findall(r'^check contracts/generated/',(ROOT/'schema-check.log').read_text(),re.M)) == 74
assert len(re.findall(r'^check .+\.ts$',(ROOT/'types-check.log').read_text(),re.M)) == 74
contracts = json.loads((ROOT/'contracts.log').read_text())
assert (contracts['schemas'],contracts['negativeMutations']) == (74,9)
expected = {
    'parity':dict(catalogs=19,invalidRequests=6,pairedCalls=50,noOverwriteChecks=19),
    'reference':dict(cases=19,payloads=9,references=12,requestFailures=6),
    'regressions':dict(requests=307,pixelOutputs=21),
    'recent-regressions':dict(requests=211,priorStableResponses=204,textPages=124,textGeometry=41,imageRequests=39,jsonRejections=7),
}
reports = {}
for name, marker in expected.items():
    # A missing or empty in-flight log can never be treated as completion.
    assert json.loads((ROOT/(name+'.log')).read_text()) == marker
    reports[name] = json.loads((ROOT/(name+'.json')).read_text())
    counts = reports[name]['counts']
    assert all(counts[k] == v for k,v in marker.items())

records = 0
def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'} <= value.keys():
            assert entry(value['path']) == value,value['path']; records += 1
        for v in value.values(): audit(v)
    elif isinstance(value,list):
        for v in value:audit(v)
audit(reports)
audit(old['previousReleaseArtifactsVerifiedUnchanged'])
audit(old['standardInputs'])
for build in old['componentBuilds'].values():
    audit(build['sourceFiles']);audit(build['artifacts'])
components = ['.codex-work/image-brush/component/mo-skia.wasm','.codex-work/image-brush/ts-raster/index.js',
              '.codex-work/harfbuzz/release/mo-hb.wasm','.codex-work/text-component/index.js']

markdown,links = 0,0
for name in sorted(sources):
    p = Path(name)
    if p.suffix != '.md':continue
    markdown += 1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        destination = unquote(target.split('#')[0].split('?')[0])
        if not destination:continue
        resolved = p.parent/destination
        assert resolved.exists() or resolved.resolve() == OUTPUT.resolve(),(name,target)
        links += 1
subprocess.run(['git','diff','--check'],check=True)
report = dict(format='musteroffice.source-images-verification/1',previousEvidence=entry(PREVIOUS),
    scope='PPTX inherited image relationships, explicit source policy, encoded resource extraction, Native CLI/Rust WASM. Does not decode or render source images.',
    checks=dict(rustTests=515,newRustTests=10,strictClippy=True,rustfmt=True,schemasChecked=74,
                generatedTypeScriptAndTypeCheck=True,genericNegativeMutations=9,
                imageCatalogs=19,invalidImageRequests=6,sourceImagePairedCalls=50,
                noOverwriteChecks=19,independentPayloads=9,independentReferences=12,
                priorPairedRuntimeRequests=518,currentPairedRuntimeRequests=568,
                priorSavedResponsesAndPixelsUnchanged=True,sourceMarkdown=markdown,localLinks=links),
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,newTestNames=new_tests,
    validationLogs=[entry(ROOT/(n+'.log')) for n in logs],
    # Keep detailed records in independently hashed reports instead of copying
    # every prior nested corpus into each later seal.
    reports={n:dict(record=entry(ROOT/(n+'.json')),counts=v['counts']) for n,v in reports.items()},
    artifacts=[entry(p) for p in ['target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker',
               'target/wasm32-unknown-unknown/debug/mo_wasm.wasm',ROOT/'wasm-node/mo_wasm.js',
               ROOT/'wasm-node/mo_wasm_bg.wasm',ROOT/'wasm-node/mo_wasm.d.ts',ROOT/'wasm-node/mo_wasm_bg.wasm.d.ts',ROOT/'wasm-node/package.json']],
    unchangedComponents=[entry(p) for p in components],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],standardInputs=old['standardInputs'],
    limitations=[
        'Catalog inspected is not all-target readiness. External and unresolved targets remain explicit; extraction includes only listed internal resources and may yield zero bytes.',
        'MIME is a package declaration. No decoder, orientation, ICC, source-image painting, visual reference or Office/WPS acceptance is claimed.',
        'Explicit external-source requests do not fetch. Authorized immutable external handles and production Artifact integration remain unimplemented.',
        '64 MiB is the encoded bundle budget, not decoded pixels, RSS or a full-document memory budget. No new product performance or package size measurement.',
        'Runtime evidence has 50 new paired inspection/extraction calls plus 518 regressions. Duplicate JSON member rejection is checked at runtime, not by decoded-object JSON Schema.',
        'Complete editing/export/player/advanced content, production Agent surfaces and Musterwork E0-E3 acceptance remain incomplete.',
    ])
raw = json.dumps(report,ensure_ascii=False,indent=2)+'\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
elif OUTPUT.exists():
    assert OUTPUT.read_text() == raw,'sealed evidence differs from current inputs'
print(json.dumps(dict(sources=len(sources),changed=len(changed),added=len(added),boundRecords=records,checks=report['checks'],evidence=entry(OUTPUT) if OUTPUT.exists() else None)))
