"""Build the pinned PNG/JPEG dependencies offline; never resolve system codecs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
from native_platform import native_platform


def record(path):
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


p = argparse.ArgumentParser()
p.add_argument('--target', choices=['native', 'wasm'], required=True)
p.add_argument('--directory', type=Path, required=True)
p.add_argument('--archives', type=Path, required=True)
p.add_argument('--emsdk', type=Path, default=Path('.codex-work/emsdk'))
p.add_argument('--jobs', type=int, default=8)
p.add_argument('--sanitize', action='store_true')
a = p.parse_args()
assert 1 <= a.jobs <= 32 and (not a.sanitize or a.target == 'native')
root = Path.cwd()
component = root / 'components/image-codec'
lock = json.loads((component / 'lock.json').read_text())
directory = a.directory.resolve()
directory.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
emsdk = a.emsdk.resolve()
env['EM_CONFIG'] = str(emsdk / '.emscripten')
cmake = Path(shutil.which('cmake')).resolve()
commands = []
log = directory / 'build.log'
log.write_text('')


def run(args):
    args = [str(x) for x in args]
    commands.append(args)
    with log.open('a') as output:
        subprocess.run(args, check=True, env=env, stdout=output, stderr=output)


flags = '-O3 -ffp-contract=off -DNO_GETENV -DNO_PUTENV'
if a.sanitize:
    flags += ' -g1 -fno-omit-frame-pointer -fsanitize=address,undefined'
common = ['-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_POSITION_INDEPENDENT_CODE=ON',
          '-DCMAKE_C_FLAGS=' + flags]
if a.target == 'wasm':
    flags += ' -sSUPPORT_LONGJMP=wasm'
    common[-1] = '-DCMAKE_C_FLAGS=' + flags
    common += ['-DCMAKE_TOOLCHAIN_FILE=' + str(emsdk / 'upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake')]
else:
    common += ['-DCMAKE_C_COMPILER=clang']
libraries, includes = {}, {}
for entry in lock['dependencies']:
    archive = a.archives / entry['archive']
    actual = record(archive)
    assert all(actual[k] == entry[k] for k in ['sha256', 'byteLength']), entry['name']
    with tarfile.open(archive) as t:
        t.extractall(directory / 'source', filter='data')
    source = directory / 'source' / entry['sourceDirectory']
    for license in entry['licenses']:
        assert record(source / license['sourcePath'])['sha256'] == license['sha256']
        assert record(component / license['path'])['sha256'] == license['sha256']
    name = entry['name']
    build = directory / name
    if name == 'zlib':
        options = ['-DZLIB_BUILD_SHARED=OFF', '-DZLIB_BUILD_STATIC=ON',
                   '-DZLIB_BUILD_TESTING=OFF', '-DZLIB_INSTALL=OFF']
        target, archive_name = 'zlibstatic', 'libz.a'
        include = [source, build]
    elif name == 'libjpeg-turbo':
        options = ['-DENABLE_SHARED=OFF', '-DENABLE_STATIC=ON', '-DWITH_TURBOJPEG=OFF',
                   '-DWITH_TOOLS=OFF', '-DWITH_TESTS=OFF',
                   '-DWITH_SIMD=' + ('OFF' if a.target == 'wasm' else 'ON')]
        target, archive_name = 'jpeg-static', 'libjpeg.a'
        include = [source / 'src', build]
    else:
        options = ['-DPNG_SHARED=OFF', '-DPNG_STATIC=ON', '-DPNG_TESTS=OFF', '-DPNG_TOOLS=OFF',
                   '-DZLIB_LIBRARY=' + str(libraries['zlib']),
                   '-DZLIB_INCLUDE_DIR=' + str(includes['zlib'][0])]
        # The source header includes the configured zconf.h from its build tree.
        options += ['-DCMAKE_C_FLAGS=' + flags + ' -I' + str(includes['zlib'][1])]
        target, archive_name = 'png_static', 'libpng16.a'
        include = [source, build]
    run([cmake, '-S', source, '-B', build, *common, *options])
    run([cmake, '--build', build, '--target', target, '-j', a.jobs])
    library = build / archive_name
    assert library.is_file(), library
    libraries[name], includes[name] = library, include
result = dict(format='musteroffice.image-codec-build/1', target=a.target, sanitizers=a.sanitize,
              lock=lock, commands=commands, cmake=record(cmake),
              cmakeVersion=subprocess.check_output([cmake, '--version'], text=True),
              script=record(Path(__file__).resolve()),
              libraries={k: record(v) for k, v in libraries.items()},
              includes={k: [str(v) for v in vs] for k, vs in includes.items()},
              headers=[record(p) for paths in includes.values() for parent in paths
                       for p in sorted(parent.glob('*.h'))],
              configurations=[record(directory / k / 'CMakeCache.txt') for k in libraries])
if a.target == 'native':
    result['nativePlatform'] = native_platform()
(directory / 'build.json').write_text(json.dumps(result, indent=2).replace(str(root) + '/', '') + '\n')
print(json.dumps({'target': a.target, 'libraries': result['libraries']}))
