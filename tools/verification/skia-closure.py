"""Audit the actual pinned Skia target, not every available GN target."""
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import sys

root = Path('.codex-work/skia')
output = Path(sys.argv[1] if len(sys.argv)>1 else '.codex-work/miter-clip/compiled-closure.json')
prior = json.loads(Path('docs/reviews/evidence/2026-09-24-skia-component-verification.json').read_text())['compiledTranslationUnits']
result = {}
for target in ['native', 'wasm']:
    source = next((root / ('source-'+target)).glob('skia-*'))
    build = source/'out/mo'
    commands = subprocess.check_output([str(root/'tools/ninja'), '-C', str(build), '-t', 'commands', 'skia'], text=True).splitlines()
    files = set()
    for command in commands:
        words = shlex.split(command)
        if '-c' in words:
            p = (build/words[words.index('-c')+1]).resolve()
            files.add(str(p.relative_to(source.resolve())))
    assert sorted(files) == [v['path'] for v in prior[target]['translationUnits']]
    entries = []
    for name in sorted(files):
        b = (source/name).read_bytes()
        entries.append({'path':name, 'byteLength':len(b), 'sha256':hashlib.sha256(b).hexdigest()})
    changed = [a['path'] for a,b in zip(entries,prior[target]['translationUnits']) if a!=b]
    assert changed == ['src/core/SkStroke.cpp']
    result[target] = {'commands':len(commands), 'translationUnits':entries, 'changedUpstreamDrawingUnits':changed}
result['dynamicLibraries'] = {}
for p in ['.codex-work/skia/mo-skia-probe', 'target/release/mo-raster-worker']:
    lines = subprocess.check_output(['otool','-L',p],text=True).splitlines()[1:]
    assert {v.strip().split(' (')[0] for v in lines} == {'/usr/lib/libc++.1.dylib','/usr/lib/libSystem.B.dylib'}
    result['dynamicLibraries'][p] = lines
output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:len(v['translationUnits']) for k,v in result.items() if k!='dynamicLibraries'}))
