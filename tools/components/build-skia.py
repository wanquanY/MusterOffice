"""Offline, pinned CPU path component build. Fetch archives/tools explicitly first.

Upstream source is re-extracted for each target. Maintained extensions add
explicit SDK controls and a private CPU join callback; original joins are intact.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tarfile


def digest(path):
    data = path.read_bytes()
    return {"path": str(path), "byteLength": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


p = argparse.ArgumentParser()
p.add_argument('--target', choices=['native', 'wasm'], required=True)
p.add_argument('--directory', type=Path, default=Path('.codex-work/skia'))
p.add_argument('--gn', type=Path, default=Path('.codex-work/skia/gn-source/out/gn'))
p.add_argument('--ninja', type=Path, default=Path('.codex-work/skia/tools/ninja'))
p.add_argument('--emsdk', type=Path, default=Path('.codex-work/emsdk'))
p.add_argument('--jobs', type=int, default=8)
p.add_argument('--sanitize', action='store_true')
p.add_argument('--image-codecs', type=Path, help='verified optional codec dependency build directory')
a = p.parse_args()
require(1 <= a.jobs <= 32, 'jobs must be between 1 and 32')
require(not a.sanitize or a.target == 'native', 'sanitizers currently require native')
require(a.target != 'native' or platform.system() == 'Darwin', 'native build currently verified for macOS only')
root = Path.cwd()
component = root / 'components/skia'
lock = json.loads((component / 'lock.json').read_text())
profile = json.loads((component / 'cpu-profile.json').read_text())
directory = a.directory.resolve()
directory.mkdir(parents=True, exist_ok=True)
archive = directory / f"skia-{lock['commit']}.tar.gz"
record = digest(archive)
require(record['sha256'] == lock['sha256'] and record['byteLength'] == lock['byteLength'], 'Skia archive mismatch')
gn, ninja, emsdk = a.gn.resolve(), a.ninja.resolve(), a.emsdk.resolve()
gn_version = subprocess.check_output([str(gn), '--version'], text=True).strip()
ninja_version = subprocess.check_output([str(ninja), '--version'], text=True).strip()
require(gn_version == lock['gnVersion'] and ninja_version == lock['ninjaVersion'], 'Build tool version mismatch')
source_root = directory / ('source-' + a.target + ('-asan' if a.sanitize else ''))
with tarfile.open(archive) as t:
    t.extractall(source_root, filter='data')
source = source_root / f"skia-{lock['commit']}"
for license_record in lock['licenses']:
    require(digest(source / license_record['path'])['sha256'] == license_record['sha256'], 'Skia license input mismatch')
for patch in ['explicit-emsdk.patch', 'custom-stroke-join.patch', 'deterministic-raster.patch', 'image-domain.patch', 'gradient-plane.patch', 'office-gradient.patch', 'rect-gradient.patch', 'elliptic-gradient.patch', 'scan-continuation.patch']:
    subprocess.run(['patch', '-p1', '--batch', '--fuzz=0', '-i', str(component / patch)],
                   cwd=source, check=True)
(source / 'src/opts/mo_image_domain_stage.h').write_bytes((component / 'mo_image_domain_stage.h').read_bytes())
(source / 'src/opts/mo_gradient_plane_stage.h').write_bytes((component / 'mo_gradient_plane_stage.h').read_bytes())
(source / 'src/opts/mo_office_gradient_stage.h').write_bytes((component / 'mo_office_gradient_stage.h').read_bytes())
(source / 'src/core/mo_raster_task.h').write_bytes((component / 'mo_raster_task.h').read_bytes())
(source / 'src/core/mo_raster_storage.h').write_bytes((component / 'mo_raster_storage.h').read_bytes())
env = os.environ.copy()
env['EM_CONFIG'] = str(emsdk / '.emscripten')
if a.target == 'wasm':
    require(lock['emsdkRelease'] in (emsdk / 'upstream/.emsdk_version').read_text(), 'SDK release mismatch')
    require(json.loads((emsdk / 'upstream/emscripten/emscripten-version.txt').read_text()) == lock['emsdkVersion'], 'SDK version mismatch')
    for entry in lock['emsdkTools']:
        require(digest(emsdk / entry['path'])['sha256'] == entry['sha256'], 'SDK tool mismatch: ' + entry['path'])
    profile.update(target_cpu='wasm', skia_emsdk_dir=str(emsdk))
profile.update(cc='clang', cxx='clang++', ar=str(emsdk / 'upstream/bin/llvm-ar'))
if a.sanitize:
    profile['extra_cflags'] += ['-g1', '-fno-omit-frame-pointer', '-fsanitize=address,undefined']
output = source / 'out/mo'
commands = []
log_path = directory / (a.target + ('-asan' if a.sanitize else '') + '-build.log')


def run(command, cwd=root):
    command = [str(x) for x in command]
    commands.append({'cwd': str(cwd), 'argv': command})
    with log_path.open('a') as log:
        subprocess.run(command, cwd=cwd, env=env, stdout=log, stderr=log, check=True)


log_path.write_text('')
codec_record, codec_flags, codec_libraries, codec_sources = None, [], [], []
if a.image_codecs:
    from image_codec_build import configure
    codec_record, codec_flags, codec_libraries, codec_sources = configure(
        root, source, a.image_codecs.resolve(), a.target, a.sanitize, profile, run)
run([gn, 'gen', output, '--fail-on-unused-args',
     '--args=' + '\n'.join(k + ' = ' + json.dumps(v) for k, v in profile.items())], source)
run([ninja, '-C', output, 'skia', '-j', a.jobs], source)
compiler = 'clang++' if a.target == 'native' else str(emsdk / 'upstream/emscripten/em++')
common = ['-std=c++20', '-O3', '-DNDEBUG', '-fno-exceptions', '-fno-rtti', '-ffp-contract=off',
          '-DSK_DISABLE_TRACING', '-I', component, '-I', source, *codec_flags]
if a.sanitize:
    common += ['-g1', '-fno-omit-frame-pointer', '-fsanitize=address,undefined']
elliptic_sources = [component / name for name in ['mo_elliptic_field.cpp', 'mo_elliptic_prepared.cpp', 'mo_gradient_coordinates.cpp']]
if a.target == 'native':
    library = output / 'libskia.a'
    probe = directory / ('mo-skia-probe-asan' if a.sanitize else 'mo-skia-probe')
    run([compiler, *common, component / 'mo_skia.cpp', component / 'mo_miter_clip.cpp', component / 'mo_gradient.cpp', component / 'mo_gradient_plane.cpp', component / 'mo_office_gradient.cpp', *elliptic_sources, component / 'mo_image.cpp', component / 'mo_image_domain.cpp', *codec_sources, root / 'tools/verification/skia-probe.cpp',
         library, *codec_libraries, '-Wl,-dead_strip', '-Wl,-map,' + str(directory / ('native-asan-link.map' if a.sanitize else 'native-link.map')), '-o', probe])
    artifacts = [library, probe]
    adapter_object = directory / ('mo-skia-adapter-asan.o' if a.sanitize else 'mo-skia-adapter.o')
    adapter_archive = directory / ('libmo_skia_adapter_asan.a' if a.sanitize else 'libmo_skia_adapter.a')
    run([compiler, *common, '-c', component / 'mo_skia.cpp', '-o', adapter_object])
    join_object = directory / ('mo-miter-clip-asan.o' if a.sanitize else 'mo-miter-clip.o')
    run([compiler, *common, '-c', component / 'mo_miter_clip.cpp', '-o', join_object])
    gradient_object = directory / ('mo-gradient-asan.o' if a.sanitize else 'mo-gradient.o')
    run([compiler, *common, '-c', component / 'mo_gradient.cpp', '-o', gradient_object])
    plane_object = directory / ('mo-gradient-plane-asan.o' if a.sanitize else 'mo-gradient-plane.o')
    run([compiler, *common, '-c', component / 'mo_gradient_plane.cpp', '-o', plane_object])
    office_object = directory / ('mo-office-gradient-asan.o' if a.sanitize else 'mo-office-gradient.o')
    run([compiler, *common, '-c', component / 'mo_office_gradient.cpp', '-o', office_object])
    elliptic_objects = []
    for source_file in elliptic_sources:
        obj = directory / (source_file.stem + ('-asan.o' if a.sanitize else '.o'))
        run([compiler, *common, '-c', source_file, '-o', obj])
        elliptic_objects.append(obj)
    image_object = directory / ('mo-image-asan.o' if a.sanitize else 'mo-image.o')
    run([compiler, *common, '-c', component / 'mo_image.cpp', '-o', image_object])
    domain_object = directory / ('mo-image-domain-asan.o' if a.sanitize else 'mo-image-domain.o')
    run([compiler, *common, '-c', component / 'mo_image_domain.cpp', '-o', domain_object])
    codec_objects = []
    for codec_source in codec_sources:
        obj = directory / (codec_source.stem + ('-asan.o' if a.sanitize else '.o'))
        run([compiler, *common, '-c', codec_source, '-o', obj])
        codec_objects.append(obj)
    adapter_archive.unlink(missing_ok=True)
    run([emsdk / 'upstream/bin/llvm-ar', 'rcsD', adapter_archive, adapter_object, join_object, gradient_object, plane_object, office_object, *elliptic_objects, image_object, domain_object, *codec_objects])
    artifacts.append(adapter_archive)
else:
    library = output / 'libskia.wasm.a'
    module = directory / 'mo-skia.mjs'
    exports = ['_mo_skia_snapshot_scopes_abi', '_mo_skia_opacity_groups_abi', '_mo_skia_elliptic_gradients_abi', '_mo_skia_rect_gradients_abi', '_mo_skia_office_gradients_abi', '_mo_skia_gradient_planes_abi', '_mo_skia_compositing_abi', '_mo_skia_clips_abi', '_mo_skia_raster_images', '_mo_skia_images_abi', '_mo_skia_raster', '_mo_skia_free', '_mo_skia_abi', '_malloc', '_free']
    exports += ['_mo_skia_execution_abi', '_mo_skia_raster_begin', '_mo_skia_raster_step', '_mo_skia_raster_take', '_mo_skia_raster_drop']
    if codec_sources:
        exports += ['_mo_image_decode', '_mo_image_decode_abi']
    # Keep auditable import names. Emscripten -O3 enables JS/wasm meta-DCE and
    # renames imports; the library itself remains compiled with -O3.
    run([compiler, *common, '-O2', '-DSKVX_DISABLE_SIMD', '-DSK_FORCE_8_BYTE_ALIGNMENT',
         component / 'mo_skia.cpp', component / 'mo_miter_clip.cpp', component / 'mo_gradient.cpp', component / 'mo_gradient_plane.cpp', component / 'mo_office_gradient.cpp', *elliptic_sources, component / 'mo_image.cpp', component / 'mo_image_domain.cpp', *codec_sources, library, *codec_libraries, '--no-entry', '-sMODULARIZE=1', '-sEXPORT_ES6=1',
         '-sENVIRONMENT=node,web', '-sFILESYSTEM=0', '-sALLOW_MEMORY_GROWTH=1',
         '-sMAXIMUM_MEMORY=268435456', '-sSTACK_SIZE=1048576', '-sABORTING_MALLOC=0',
         '-sDYNAMIC_EXECUTION=0',
         '-sEXPORTED_FUNCTIONS=' + json.dumps(exports),
         '-sEXPORTED_RUNTIME_METHODS=["HEAPU8","HEAPU32"]', '-o', module])
    artifacts = [library, module, module.with_suffix('.wasm')]
targets = subprocess.check_output([str(gn), 'desc', str(output), '*', '--format=json'], cwd=source, env=env)
target_file = directory / (a.target + ('-asan' if a.sanitize else '') + '-targets.json')
target_file.write_bytes(targets)
result = {'format': 'musteroffice.skia-build/1', 'target': a.target, 'sanitizers': a.sanitize,
          'lock': lock, 'profile': profile, 'commands': commands,
          'compiler': subprocess.check_output([compiler, '--version'], env=env, text=True),
          'gn': digest(gn), 'ninja': digest(ninja), 'targetGraph': digest(target_file),
          'componentSources': [digest(p) for p in sorted(component.iterdir()) if p.is_file()],
          'artifacts': [digest(p) for p in artifacts]}
if codec_record:
    result['imageCodecs'] = codec_record
    result['componentSources'] += [digest(p) for p in sorted((root / 'components/image-codec').rglob('*')) if p.is_file()]
    result['componentSources'] += [digest(root / 'tools/components/image_codec_build.py')]
    result['artifacts'] += [digest(p) for p in codec_libraries]
# Paths in the build record remain repository relative, including command args.
text = json.dumps(result, indent=2).replace(json.dumps(str(root)), '"."').replace(str(root) + '/', '') + '\n'
(directory / (a.target + ('-asan' if a.sanitize else '') + '-build.json')).write_text(text)
print(json.dumps({'target': a.target, 'artifacts': json.loads(text)['artifacts']}))
