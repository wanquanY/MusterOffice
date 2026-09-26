"""Record actual candidate SDK protocol behavior without claiming MCP delivery."""
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote

root=Path('.codex-work/mcp-native')
parent_path=Path('docs/reviews/evidence/2026-09-26-operation-discovery-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-mcp-sdk-evaluation.json')
def entry(path):
    p=Path(path); data=p.read_bytes()
    return dict(path=str(p),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())

assert entry(parent_path)['sha256']=='f7dd25e4877725b8233220e2ae09f7653634b4117cd5b9002cf7165b29d55c08'
parent=json.loads(parent_path.read_text()); prior={r['path']:r for r in parent['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
assert changed=={'docs/README.md','docs/implementation/progress.md','docs/implementation/dependencies.md'},changed
added={
    'components/rmcp/component.json','components/rmcp/LICENSE-UPSTREAM.txt',
    'tools/experiments/mcp-sdk-probe/Cargo.toml','tools/experiments/mcp-sdk-probe/Cargo.lock',
    'tools/experiments/mcp-sdk-probe/src/main.rs','tools/experiments/mcp-sdk-probe/check.py',
    'tools/experiments/mcp-sdk-probe/README.md','docs/implementation/mcp-sdk-evaluation.md',
    'tools/verification/mcp-sdk-evidence.py',
}
assert not added & set(prior)
component=json.loads(Path('components/rmcp/component.json').read_text())
assert component['status']=='evaluated-candidate-not-production-adopted'
assert component['version']=='3.4.0' and not component['defaultFeatures']
assert entry(root/'rmcp-3.4.0.crate')['sha256']==component['archive']['sha256']
assert entry('components/rmcp/'+component['license']['path'])['sha256']==component['license']['sha256']
lock=tomllib.loads(Path('tools/experiments/mcp-sdk-probe/Cargo.lock').read_text())
rmcp=next(p for p in lock['package'] if p['name']=='rmcp')
assert rmcp['checksum']==component['archive']['sha256']
assert 'rmcp' not in {p['name'] for p in tomllib.loads(Path('Cargo.lock').read_text())['package']}
history=json.loads((root/'probe-checks.json').read_text()); checks={r['name']:r for r in history}
assert set(checks)=={'probe-fmt','probe-clippy','probe-tree','probe-metadata','probe-release'}
for check in checks.values():
    assert check['exitCode']==0
    assert not any(bad in Path(check['log']).read_text() for bad in ['error:','FAILED','Traceback'])
reference_path=root/'protocol-release-3/report.json'
reference=json.loads(reference_path.read_text())
binary=entry(reference['binary']['path'])
assert binary['sha256']==reference['binary']['sha256']
assert reference['inputByteLimit']==4096 and reference['overBudgetExit']!=0
assert [(s['era'],len(s['calls']),s['schemaPairs'],s['rejectedQueries'],s['rejectedMetadata']) for s in reference['sessions']]==[
    ('legacy',15,10,3,0),('modern',17,10,3,2)]
expected_path=Path('.codex-work/operation-discovery/reference-1/schemas.json')
assert entry(expected_path)==next(r for r in parent['artifacts'] if r['path']==str(expected_path))
expected={d['id']:d for d in json.loads(expected_path.read_text())}
matched=0
for p in reference_path.parent.glob('*.response.json'):
    result=json.loads(p.read_text()).get('result',{})
    if 'structuredContent' not in result: continue
    document=result['structuredContent']
    assert document==expected[document['id']]
    assert json.loads(result['content'][0]['text'])==document
    matched+=1
assert matched==20
metadata=json.loads(Path(checks['probe-metadata']['log']).read_text())
active={n['id'] for n in metadata['resolve']['nodes']}
dependencies=sorted([dict(name=p['name'],version=p['version'],declaredLicense=p['license'],source=p['source'])
    for p in metadata['packages'] if p['id'] in active and p['source']],key=lambda p:(p['name'],p['version']))
assert len(dependencies)==77
rmcp_node=next(n for n in metadata['resolve']['nodes'] if n['id'].endswith('#rmcp@3.4.0'))
assert all(f not in rmcp_node['features'] for f in ['default','macros','client','base64','server-side-http','auth'])
sources=set(prior)|added; links=0
for name in sources:
    p=Path(name)
    if p.suffix in ['.rs','.py','.ts','.mjs','.cpp','.h']: assert len(p.read_text().splitlines())<=2000,name
    if p.suffix!='.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'): continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:
            destination=(p.parent/target).resolve()
            assert destination.exists() or destination==output.resolve(),(name,target)
            links+=1
artifacts={str(p) for p in root.glob('*.log')}|{str(root/'probe-checks.json'),str(root/'rustc-version.txt'),str(root/'rmcp-3.4.0.crate'),str(root/'candidate-source.json')}
for dirname in ['protocol-1','protocol-release-1','protocol-release-2','protocol-release-3']:
    artifacts|={str(p) for p in (root/dirname).rglob('*') if p.is_file()}
artifacts|={binary['path'],str(expected_path)}
report=dict(format='musteroffice.mcp-sdk-evaluation/1',previousEvidence=entry(parent_path),
    scope='Standalone SDK selection experiment, not a production MCP server or E0-6 acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    buildChecks=list(checks.values()),buildAttemptHistory=history,
    runtimeReference=entry(reference_path),expectedSchemaDocuments=entry(expected_path),
    checks=dict(protocolVersions=['2025-11-25','2026-07-28'],stdioRequests=32,realKernelSchemaPairs=20,
        rejectedToolQueries=6,rejectedProtocolMetadata=2,oversizeInputRejected=True,inputByteLimit=4096,
        strictClippy=True,fmt=True,priorComputationalSourcesUnchanged=True,priorMainLockUnchanged=True,
        productionMcpAdapter=False,durableScheduler=False,httpTransport=False,musterworkIntegration=False,localLinks=links),
    candidate=component,selectedSdkFeatures=rmcp_node['features'],nativeDependencyDeclarations=dependencies,
    nativeProbe=dict(binary=binary,target='aarch64-apple-darwin',toolchain=entry(root/'rustc-version.txt'),
        scope='Release executable contains the actual shared schema computation. Not an SDK-only delta, installer, office runtime, complete dependency distribution or benchmark.'),
    artifacts=[entry(p) for p in sorted(artifacts)],
    findings=[
        'Actual legacy initialize and modern per-request discovery/tool messages return identical typed kernel SchemaDocuments.',
        'Upstream default AsyncRwTransport buffers unbounded lines. This probe uses the SDK framed codec with an explicit byte limit; queued tasks, outputs and full memory budgets remain separate requirements.',
        'The missing-metadata negative initially expected -32600. The official 2026 basic protocol requires -32602; the SDK was correct. The final test preserves an exact error-code assertion and the earlier failing response is retained.',
        'The upstream Cargo license declaration is Apache-2.0, while the full LICENSE preserves an MIT-to-Apache transition notice and both texts. Full distribution review remains required.',
        'The independent Cargo workspace prevents SDK and Tokio dependencies from entering the computational native/WASM graph.',
    ],
    limitations=[
        'This probe implements only schema discovery, not office tools, resource transfer, task projection, scheduling, HTTP, trusted-identity injection or a production concurrency policy.',
        'No complete SDK conformance or third-party AI application acceptance is claimed.',
        'Input frame length is tested; RSS, output limits, concurrent flood, queue bounds and graceful shutdown require production runtime work.',
        'The main computational sources are unchanged, so historical workspace test counts are not represented as new executions.',
        'All original complete presentation, Native/WASM, Agent, Office/WPS, performance, package and Musterwork replacement requirements remain open.',
    ])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv: assert output.read_text()==encoded,'candidate evidence changed'
else:
    with output.open('x') as f: f.write(encoded)
print(json.dumps(dict(evidence=entry(output),checks=report['checks'],sourceFiles=len(sources),nativeDependencies=len(dependencies),probeBinaryBytes=binary['byteLength'])))
