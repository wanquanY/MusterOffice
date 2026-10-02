"""Explicit dependency bridge for the optional SkCodec extension."""
import hashlib
import json
from pathlib import Path
from native_platform import native_platform


def configure(root, source, directory, target, sanitize, profile, run):
    component = root / 'components/image-codec'
    record = json.loads((directory / 'build.json').read_text())
    assert record['format'] == 'musteroffice.image-codec-build/1'
    assert record['target'] == target and record['sanitizers'] == sanitize
    if target == 'native':
        current = native_platform()
        # Existing macOS records predate platform metadata. Linux has no legacy
        # build and must always carry the exact OS/architecture identity.
        assert record.get('nativePlatform', current if current['os'] == 'macos' else None) == current
    assert record['lock'] == json.loads((component / 'lock.json').read_text())
    for entry in [record['script'], *record['libraries'].values(), *record['headers'],
                  *record['configurations']]:
        data = (root / entry['path']).read_bytes()
        assert len(data) == entry['byteLength'] and hashlib.sha256(data).hexdigest() == entry['sha256']
    run(['patch', '-p1', '--batch', '--fuzz=0', '-i', component / 'bounded-png.patch'], source)
    # Replace just the selected dependency build descriptions in this extracted
    # source tree. No library-name search or system headers survive this bridge.
    targets = [('libpng', 'libpng'), ('libjpeg-turbo', 'libjpeg'), ('zlib', 'zlib')]
    for name, target_name in targets:
        includes = [str(root / p) for p in record['includes'][name]]
        library = str(root / record['libraries'][name]['path'])
        text = ('# MusterOffice generated explicit pinned dependency bridge.\n'
                'import("../third_party.gni")\n'
                + 'system(' + json.dumps(target_name) + ') {\n'
                + '  include_dirs = ' + json.dumps(includes) + '\n'
                + '  libs = [' + json.dumps(library) + ']\n}\n')
        (source / 'third_party' / name / 'BUILD.gn').write_text(text)
    profile.update(skia_use_libpng_decode=True, skia_use_libjpeg_turbo_decode=True,
                   skia_use_zlib=True, skia_use_jpeg_gainmaps=False)
    if target == 'wasm':
        profile['extra_cflags'] += ['-sSUPPORT_LONGJMP=wasm']
    # libpng's build needs the generated zconf header as well as zlib.h.
    includes = [root / p for values in record['includes'].values() for p in values]
    flags = [part for path in includes for part in ['-I', str(path)]]
    if target == 'wasm':
        flags += ['-sSUPPORT_LONGJMP=wasm']
    libraries = [root / record['libraries'][name]['path'] for name in ['libpng', 'libjpeg-turbo', 'zlib']]
    return record, flags, libraries, [component / 'mo_decode.cpp', component / 'mo_envelope.cpp']
