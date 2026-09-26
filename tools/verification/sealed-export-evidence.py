"""Bind stream/seal implementation, real public calls and scoped measurements."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote

root=Path('.codex-work/sealed-export')
parent_path=Path('docs/reviews/evidence/2026-09-26-resource-host-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-sealed-export-verification.json')
def entry(path):
    p=Path(path);b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(parent_path)['sha256']=='b654adc6fbdd035ebb6f55e46460295d5d4855ae188e177c17c0b79d7d5fea66'
parent=json.loads(parent_path.read_text());prior={r['path']:r for r in parent['sourceFiles']}
allowed=set('''Cargo.lock
Cargo.toml
README.md
crates/mo-kernel-api/src/package.rs
crates/mo-kernel-api/src/pptx.rs
crates/mo-opc/src/lib.rs
crates/mo-opc/src/writer.rs
crates/mo-pptx/src/lib.rs
crates/mo-pptx/src/write.rs
crates/mo-standard-host/Cargo.toml
crates/mo-standard-host/src/lib.rs
crates/mo-standard-host/tests/resources.rs
docs/README.md
docs/design/agent-interfaces.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-cli/Cargo.toml
tools/mo-cli/src/artifact.rs
tools/mo-cli/src/pptx.rs'''.splitlines())
changed={p for p,r in prior.items() if entry(p)!=r}
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''crates/mo-opc/src/sink.rs
crates/mo-opc/tests/sealed_output.rs
crates/mo-pptx/tests/stream_export.rs
crates/mo-native-io/Cargo.toml
crates/mo-native-io/src/lib.rs
crates/mo-native-io/tests/spool.rs
tools/mo-cli/tests/stream_export.rs
docs/implementation/sealed-export.md'''.splitlines())
added|={str(p) for p in Path('tools/verification').glob('sealed-export-*') if p.is_file()}
assert not added&set(prior)
sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.py','.mjs','.ts','.cpp','.h']:
        assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text())
assert len(checks)==12 and all(c['exitCode']==0 for c in checks)
for c in checks:
    assert not any(bad in Path(c['log']).read_text() for bad in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
assert len(tests)==796 and set(parent['rustTestNames'])<=set(tests)
for name in ['valid_but_different_stored_package_is_rejected_by_writer_digest','corrupted_real_stored_bytes_are_rejected_and_removed','multi_megabyte_input_and_output_use_bounded_io_without_whole_file_buffers','cli_stream_export_keeps_exact_native_bytes_and_no_staging_leftovers','native_pptx_writer_consumes_scope_authorized_range_readers']:
    assert name in tests
for p,r in prior.items():
    if p.startswith(('contracts/generated/','packages/contracts/src/generated/')):assert entry(p)==r
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==104
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==104
assert 'mo-native-io' not in (root/'wasm-tree.log').read_text()
assert 'rusqlite' not in (root/'wasm-tree.log').read_text()
assert 'mo-native-io' in (root/'native-tree.log').read_text()
assert 'rusqlite' not in (root/'native-tree.log').read_text()
assert entry(root/'baseline/mo-cli')['sha256']==parent['unchangedKernelArtifacts']['nativeCli']['sha256']
parity=json.loads((root/'parity.json').read_text())
assert parity['authoredCases']==17 and parity['editorCases']==22
assert parity['previousAuthoredByteIdentical']==7 and parity['previousEditedByteIdentical']>0
assert parity['previousSourceResponsesIdentical']>0 and parity['previousEditorResponsesIdentical']==22
for r in [parity['native'],parity['wasm'],parity['parentWasm'],*parity['reports'].values()]:assert entry(r['path'])==r
benchmark=json.loads((root/'benchmark.json').read_text())
assert len(benchmark['records'])==16
for r in [*benchmark['programs'].values(),*benchmark['inputs']]:assert entry(r['path'])==r
for r in benchmark['records']:
    for key in ['output','measurement']:assert entry(r[key]['path'])==r[key]
    assert r['peakRssBytes']>0 and r['wallSeconds']>=0
independent=json.loads((root/'independent.json').read_text())
assert independent['authoredFiles']==7 and independent['sourceIndependentExitCode']==0
for p in independent['reports']:assert entry(p['path'])==p
for p in independent['programs']+independent['schemaFiles']:assert entry(p['path'])==p
publication=json.loads((root/'publication.json').read_text())
assert publication['actualCalls']==2 and publication['pixelsAndMetadataUnchanged'] and publication['existingDestinationUnchanged'] and publication['noStagingLeftovers']
for key in ['cli','worker','request','referenceResponse','referencePixels','output']:assert entry(publication[key]['path'])==publication[key]
links=0
for name in sources:
    p=Path(name)
    if p.suffix!='.md':continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:
            assert (p.parent/target).exists() or (p.parent/target).resolve()==output.resolve(),(name,target)
            links+=1
subprocess.run(['git','diff','--check'],check=True)
artifact_paths={p for p in root.rglob('*') if p.is_file() and p.name not in ['plan.md','next.md','seal.log','seal-check.log']}
for name in ['authored','source']:
    r=json.loads((root/f'{name}-parity.json').read_text())
    artifact_paths|={p for p in Path(r['artifactDirectory']).rglob('*') if p.is_file()}
current={key:entry(p) for key,p in {'nativeCli':'target/debug/mo-cli','releaseCli':'target/release/mo-cli','nativeHost':'target/debug/mo-host','releaseHost':'target/release/mo-host','rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js')}.items()}
attempts=[dict(reason='Historical source corpus expected old diagnostics for an unbound MCE prefix and malformed AlternateContent. Frozen parent and current runtime agree; only a new derived manifest changes those two expectations.',logs=[entry(root/'attempts/historical-source-expectation/parity.log'),entry(root/'attempts/historical-alternate-content/parity.log')]),dict(reason='Independent report schemasChecked is a list of checked parts, not an integer. Corrected aggregation to its length before rerunning all independent files.',logs=[entry(root/'attempts/independent-report-shape/independent.log')])]
report=dict(format='musteroffice.sealed-export-verification/1',previousEvidence=entry(parent_path),scope='Shared injected result storage, actual sealed-byte OPC verification, streaming authored PPTX/CLI and native host I/O. Not public export job, full bundle or Musterwork replacement.',sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),removedSources=[],workspaceChecks=checks,rustTestNames=tests,checks=dict(rustTests=len(tests),newRustTests=17,schemas=104,wireContractsUnchanged=True,typescript=True,strictClippy=True,nativeAndWasm=True,authoredCalls=parity['authoredCases'],sourceCalls=parity['sourceCases'],editorCalls=parity['editorCases'],previousAuthoredCandidatesByteIdentical=parity['previousAuthoredByteIdentical'],previousSourceCandidatesByteIdentical=parity['previousEditedByteIdentical'],sourceInspectionsUnchanged=parity['previousSourceResponsesIdentical'],authoredIndependentFiles=7,localLinks=links,officeWpsNewAcceptance=False),reports={k:entry(root/(k+'.json')) for k in ['parity','independent','benchmark','publication']},artifacts=[entry(p) for p in sorted(artifact_paths)],currentArtifacts=current,measurements=benchmark['summary'],size=dict(nativeCliReleaseBytes=current['releaseCli']['byteLength'],nativeHostReleaseBytes=current['releaseHost']['byteLength'],rustWasmBytes=current['rustWasm']['byteLength'],cppWasmBytes=parent['size']['cppWasmBytes'],note='Development components, not full fonts/media/runtime closure or installation size.'),limitations=['Full v0.4, E0-E3 and Musterwork goal remains active and incomplete.', 'VerifiedPackage certifies implemented OPC/ZIP/XML/digest checks, not full PresentationML XSD, layout, editability, playback or target-app quality.', 'FileSpool has per-file logical byte budget and 64 KiB buffering; XML plans, metadata, aggregate storage, RSS/process limits and browser backing storage remain separate work.', 'Normal Drop cleanup is best effort; explicit discard reports failures. Crash orphan tracking and durable job integration are not implemented.', 'Local CLI uses a verified same-file hard link and refuses overwrite. It is not the fenced Artifact/Resource/Ledger/success-receipt transaction or a power-loss guarantee.', 'Current native tests and debug benchmark run on this macOS arm64 machine. Other OS execution and release performance remain unmeasured.', 'Existing authored/source/editor public outputs replayed against prior WASM. One shared-spool publication regression uses a frozen raster worker; no new renderer or full frame replay or Office/WPS acceptance.', 'MCP/SDK/Skill/Plugin, complete advanced objects/media, public previews/quality bundle and actual Musterwork integration remain required.'])
report['attempts']=attempts
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    with output.open('x') as stream:stream.write(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),checks=report['checks'],size=report['size'],measurements=report['measurements']),ensure_ascii=False))
