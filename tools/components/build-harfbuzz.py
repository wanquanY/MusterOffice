"""Build the pinned HarfBuzz component. No implicit library selection or download.
The archive must already be fetched. All build inputs under src/ are re-extracted
from its verified contents; upstream code is kept in ignored build directories.
"""
import argparse
import hashlib
import json
import os
import shlex
from pathlib import Path
import subprocess
import tarfile
from native_platform import native_platform

p=argparse.ArgumentParser()
p.add_argument('--target',choices=['native','wasm'],required=True)
p.add_argument('--directory',type=Path,default=Path('.codex-work/harfbuzz'))
p.add_argument('--emsdk',type=Path,default=Path('.codex-work/emsdk'))
p.add_argument('--archiver',default='llvm-ar',help='LLVM archiver for deterministic native archives')
p.add_argument('--fault-tests',action='store_true')
p.add_argument('--sanitize',action='store_true')
p.add_argument('--reference',action='store_true')
a=p.parse_args();root=Path.cwd();a.directory.mkdir(parents=True,exist_ok=True)
lock=json.loads(Path('components/harfbuzz/lock.json').read_text())
archive=a.directory/f"harfbuzz-{lock['version']}.tar.xz"
data=archive.read_bytes();assert len(data)==lock['byteLength'] and hashlib.sha256(data).hexdigest()==lock['sha256']
# A separate verified tree avoids sharing mutable upstream inputs across builds.
source=a.directory/('source-'+a.target)
source.mkdir(exist_ok=True)
with tarfile.open(archive) as t:t.extractall(source,filter='data')
upstream=source/f"harfbuzz-{lock['version']}"/'src'
component=Path('components/harfbuzz')
common=['-std=c++17','-O2','-DNDEBUG','-fno-exceptions','-fno-rtti','-ffp-contract=off',
    '-DHB_NO_OPEN','-DHB_NO_GETENV','-DHB_NO_SETLOCALE','-DHB_NO_FEATURES_H','-DHB_NO_BUFFER_MESSAGE',
    '-Dhb_malloc_impl=mo_hb_alloc','-Dhb_calloc_impl=mo_hb_calloc','-Dhb_realloc_impl=mo_hb_realloc','-Dhb_free_impl=mo_hb_dealloc',
    '-include',str(component/'mo_hb_allocator.h'),'-I',str(upstream),'-I',str(component)]
if a.fault_tests:common+=['-DMO_HB_FAULT_TEST']
if a.sanitize:
    assert a.target=='native', 'sanitizer probe currently native only'
    common+=['-g1','-fno-omit-frame-pointer','-fsanitize=address,undefined']
env=os.environ.copy()
compiler='clang++' if a.target=='native' else str(a.emsdk/'upstream/emscripten/em++')
if a.target=='wasm':env['EM_CONFIG']=str((a.emsdk/'.emscripten').resolve())
version=subprocess.check_output([compiler,'--version'],env=env,text=True)
archiver_version=subprocess.check_output([a.archiver,'--version'],env=env,text=True) if a.target=='native' else None
commands=[]
def run(command):
    commands.append(command)
    with (a.directory/(a.target+'-build.log')).open('a') as log:subprocess.run(command,check=True,env=env,stdout=log,stderr=log)
if a.target=='native':
    inputs=[upstream/'harfbuzz.cc',component/'mo_hb.cpp',component/'mo_hb_metrics.cpp',component/'mo_hb_outlines.cpp',component/'mo_hb_allocator.cpp'];objects=[]
    for i,source_file in enumerate(inputs):
        target=a.directory/f'native-{i}.o';objects.append(str(target))
        run([compiler,*common,'-c',str(source_file),'-o',str(target)])
    library=a.directory/'libmo_harfbuzz.a'
    library.unlink(missing_ok=True)
    run([a.archiver,'rcsD',str(library),*objects])
    probe=a.directory/'mo-hb-probe'
    run([compiler,*common,'tools/verification/harfbuzz-probe.cpp',str(library),'-o',str(probe)])
    artifacts=[library,probe]
    if a.fault_tests:
        observer=a.directory/'fault-observer'
        run([compiler,*common,'tools/verification/harfbuzz-fault-observer.cpp',str(library),'-o',str(observer)])
        artifacts.append(observer)
    if a.reference:
        reference=a.directory/'hb-shape-reference'
        glib=shlex.split(subprocess.check_output(['pkg-config','--cflags','--libs','glib-2.0'],text=True))
        run([compiler,'-std=c++17','-O2','-DHB_NO_FEATURES_H','-I',str(upstream),str(upstream/'harfbuzz.cc'),
            str(upstream.parent/'util/hb-shape.cc'),*glib,'-o',str(reference)])
        artifacts.append(reference)
else:
    exports=['_mo_hb_shape','_mo_hb_measure_font','_mo_hb_outline_font','_mo_hb_free','_mo_hb_version','_mo_hb_alloc_live','_malloc','_free']
    if a.fault_tests:exports+=['_mo_hb_fail_after']
    module=a.directory/'mo-hb.mjs'
    run([compiler,*common,str(upstream/'harfbuzz.cc'),str(component/'mo_hb.cpp'),str(component/'mo_hb_metrics.cpp'),str(component/'mo_hb_outlines.cpp'),str(component/'mo_hb_allocator.cpp'),
        '--no-entry','-sMODULARIZE=1','-sEXPORT_ES6=1','-sENVIRONMENT=node,web','-sFILESYSTEM=0',
        '-sALLOW_MEMORY_GROWTH=1','-sMAXIMUM_MEMORY=536870912','-sSTACK_SIZE=1048576','-sABORTING_MALLOC=0',
        '-sDYNAMIC_EXECUTION=0','-sEXPORTED_FUNCTIONS='+json.dumps(exports),
        '-sEXPORTED_RUNTIME_METHODS=["HEAPU8","HEAPU32"]','-o',str(module)])
    artifacts=[module,module.with_suffix('.wasm')]
result={'format':'musteroffice.harfbuzz-build/1','target':a.target,'faultTests':a.fault_tests,'sanitizers':a.sanitize,'lock':lock,'compiler':version,'archiver':archiver_version,
 'commands':commands,'artifacts':[{'path':str(f),'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'byteLength':f.stat().st_size} for f in artifacts]}
if a.target == 'native':
    result['nativePlatform'] = native_platform()
(a.directory/(a.target+'-build.json')).write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'target':a.target,'artifacts':result['artifacts']}))
