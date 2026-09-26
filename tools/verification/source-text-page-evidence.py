"""Seal real Native source text page rendering, separate from old runtime releases."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT=Path('.codex-work/source-text-page')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-source-frame-library-verification.json')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-source-text-page-library-verification.json')
def entry(path):
    p=Path(path);b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(PREVIOUS)['sha256']=='83df353946171fbe1e902bf7747bc20a62ea0589ec450947e27f50c1493bed1f'
old=json.loads(PREVIOUS.read_text())
allowed={
    'Cargo.lock','crates/mo-harfbuzz-sys/Cargo.toml','crates/mo-kernel-api/src/pptx_page.rs',
    'crates/mo-pptx/src/source/text.rs','crates/mo-presentation-compile/src/coordinate_budget.rs',
    'crates/mo-presentation-compile/src/lib.rs','crates/mo-presentation-compile/src/source_frame.rs',
    'crates/mo-presentation-compile/src/source_page.rs','crates/mo-presentation-compile/src/source_page/layers.rs',
    'crates/mo-presentation-compile/src/source_page/types.rs','crates/mo-presentation-compile/src/source_text.rs',
    'crates/mo-presentation-compile/src/source_text/budget.rs',
    'README.md','docs/README.md','docs/implementation/development.md','docs/implementation/progress.md',
    'docs/implementation/source-frame.md',
}
changed=[p['path'] for p in old['sourceFiles'] if entry(p['path'])!=p]
assert set(changed)==allowed,(set(changed)-allowed,allowed-set(changed))
added=['crates/mo-pptx/src/source/text/paint.rs','crates/mo-presentation-compile/src/source_text_page.rs',
       *['crates/mo-presentation-compile/src/source_text_page/'+n+'.rs' for n in ['types','glyphs','precision']],
       'crates/mo-harfbuzz-sys/tests/source_text_page.rs',
       *['tools/verification/source-text-page-'+n+'.py' for n in ['reference','regressions','evidence']],
       'docs/implementation/source-text-page.md']
sources={p['path'] for p in old['sourceFiles']}|set(added)
assert len(sources)==len(old['sourceFiles'])+len(added)
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.py','.mjs','.h','.cpp']:
        assert len(Path(p).read_text().splitlines())<=2000,p
lock=Path('Cargo.lock').read_text();start=lock.index('name = "mo-harfbuzz-sys"');end=lock.index('[[package]]',start)
section=lock[start:end]
for dependency in ['mo-raster','mo-skia-sys']:
    line=' "'+dependency+'",\n';assert section.count(line)==1;section=section.replace(line,'')
restored=lock[:start]+section+lock[end:]
assert hashlib.sha256(restored.encode()).hexdigest()==next(p['sha256'] for p in old['sourceFiles'] if p['path']=='Cargo.lock')
for record in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(),*old['standardInputs'],old['nativeComponentLibrary']]:
    assert entry(record['path'])==record
logs=['native-tests.log','workspace-tests.log','clippy.log','wasm-check.log','fmt.log','schema-check.log',
      'workers-build.log','reference.log','reference-native.log','page-regressions.log','text-regressions.log']
for name in logs:
    text=(ROOT/name).read_text()
    assert not any(s in text for s in ['error:','FAILED','Traceback','AssertionError']),name
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M)
new=[s for s in tests if s not in old['rustTestNames']]
assert len(tests)==461 and len(new)==7
assert len(re.findall(r'^check contracts/generated/',(ROOT/'schema-check.log').read_text(),re.M))==68
reference=json.loads((ROOT/'reference.json').read_text())
assert reference['counts']==dict(packages=10,objects=11,glyphs=18,worldControlCoordinates=102,
                                interiorPixels=1154714,excludedEdgePixels=45286)
text_regressions=json.loads((ROOT/'regressions.json').read_text())
assert text_regressions['total']==len(text_regressions['cases'])==207
pages=json.loads((ROOT/'page-regressions.json').read_text())
assert pages['counts']==dict(source=28,compile=36,raster=36,pixels=21)
skia_build=Path('.codex-work/skia/native-build.json');skia=json.loads(skia_build.read_text())
assert skia['target']=='native' and not skia['sanitizers']
libraries=[r for r in skia['artifacts'] if r['path'].endswith('.a')]
assert len(libraries)==2
def audit(value):
    if isinstance(value,dict):
        if {'path','sha256','byteLength'}<=value.keys():assert entry(value['path'])==value,value['path']
        for v in value.values():audit(v)
    elif isinstance(value,list):
        for v in value:audit(v)
for report in [reference,text_regressions,pages,libraries]:audit(report)
report=dict(
    format='musteroffice.source-text-page-library-verification/1',previousEvidence=entry(PREVIOUS),
    scope='Native PPTX solid text paint through source frame, actual object/page transforms and HarfBuzz + Skia pixels; development Rust library, not product replacement or new wire/WASM execution.',
    checks=dict(rustTests=461,newRustTests=7,strictClippy=True,rustfmt=True,
                sourceCompilerWasmTarget='compiled-only',schemasChecked=68,schemaAndTypeScriptDelta=False,
                exactNativeTextRegressionRequests=207,exactNativeSourcePageRegressionRequests=100,
                priorPagePixelOutputsUnchanged=21,newRuntimeParityBatches=0),
    changedPreviousSources=sorted(changed),addedSources=sorted(added),sourceFiles=[entry(p) for p in sorted(sources)],
    rustTestNames=tests,newTestNames=new,validationLogs=[entry(ROOT/n) for n in logs],
    reference=entry(ROOT/'reference.json'),independentReference=reference,
    textRegressions=entry(ROOT/'regressions.json'),textRegressionEvidence=text_regressions,
    pageRegressions=entry(ROOT/'page-regressions.json'),pageRegressionEvidence=pages,
    nativeComponents=dict(harfbuzz=old['nativeComponentLibrary'],skiaBuild=entry(skia_build),skiaLibraries=libraries),
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],standardInputs=old['standardInputs'],
    dependencyChanges='Two local development edges in mo-harfbuzz-sys (mo-raster, mo-skia-sys). No runtime dependency or external version change; exact lock reconstruction verified.',
    limitations=[
        'Draft native text page profile. Old no-font wire operations retain text rejection; new Rust entry points have no new CLI/WASM/TS protocol yet.',
        'Solid/no-fill text and fontRef/phClr only. Native underline, strike, outline, highlight, nonempty effects, links and mixed-color glyph clusters still need mapping, not silently dropped.',
        'Source glyph/frame limitations remain: multi-column/vertical/autofit, native hanging punctuation and more character/paragraph features incomplete. Full phase-one scope retained.',
        'Numerical bounds are relative to selected line breaks and integer font-engine outputs; no source guide, font algorithm, break-topology, antialiasing or Office/WPS certification.',
        'Ten original simple-font PPTX fixtures validate control points and opaque interior pixels; 45286 edge pixels intentionally outside pixel oracle. No real-font visual quality, target-app edit/save/reopen or playback evidence.',
        'Shared component/result/prepared-plan budgets do not prove total RSS. Source catalogs/cascade and in-flight frame preparation remain separately bounded. No performance or installer size claims.',
        'Render releases detailed provenance/shaping plans before the raster backend and returns compact metadata plus pixels; independent verification explicitly compiles a diagnostic plan separately.',
        'Native fault/cancellation scenarios use process isolation. Production instance pools and Musterwork task/Artifact publication are not implemented by this library step.',
        'No new WASM execution, release rebuild, product integration, Agent distribution or E0-E3 completion. All eleven old release artifacts remain unchanged.',
    ])
raw=json.dumps(report,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as file:file.write(raw)
print(json.dumps(dict(sources=len(sources),checks=report['checks'],native=reference['counts'],
                      evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
