#!/usr/bin/env python3
"""Fetch verified public inputs and build the existing Linux native components.

This is CI provisioning, not an alternate component builder. Source archives are
checked before extraction; the existing builders check licenses, source identity
and output digests. It needs clang, llvm-ar, cmake, ninja, nasm, git and Python 3.13.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
DIRECTORY = ROOT / '.codex-work/ci-native'
NINJA_COMMIT = '3441b633c2fe2c494e958780ba0f4227b1327634'


def run(*args, cwd=ROOT):
    subprocess.run([str(arg) for arg in args], cwd=cwd, check=True)


def verify(path, record):
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    if path.stat().st_size != record['byteLength'] or digest != record['sha256']:
        raise ValueError(f'Pinned source mismatch: {path.name}')


def download(record, path):
    if path.exists():
        verify(path, record)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.partial')
    try:
        request = urllib.request.Request(record['url'], headers={'User-Agent': 'MusterOffice-CI'})
        with urllib.request.urlopen(request, timeout=120) as source, temporary.open('wb') as target:
            shutil.copyfileobj(source, target)
        verify(temporary, record)
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def checkout(url, revision, directory):
    directory.mkdir(parents=True, exist_ok=True)
    if not (directory / '.git').exists():
        run('git', 'init', directory)
        run('git', 'remote', 'add', 'origin', url, cwd=directory)
    run('git', 'fetch', '--depth=1', 'origin', revision, cwd=directory)
    run('git', 'checkout', '--detach', revision, cwd=directory)
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=directory, text=True).strip()
    if actual != revision:
        raise ValueError('Build tool source revision mismatch')


def main():
    DIRECTORY.mkdir(parents=True, exist_ok=True)
    skia = json.loads((ROOT / 'components/skia/lock.json').read_text())
    hb = json.loads((ROOT / 'components/harfbuzz/lock.json').read_text())
    codecs = json.loads((ROOT / 'components/image-codec/lock.json').read_text())['dependencies']
    download(skia, DIRECTORY / 'skia' / f"skia-{skia['commit']}.tar.gz")
    download(hb, DIRECTORY / 'harfbuzz' / f"harfbuzz-{hb['version']}.tar.xz")
    for record in codecs:
        download(record, DIRECTORY / 'archives' / record['archive'])
    ninja = DIRECTORY / 'tools/ninja'
    checkout('https://github.com/ninja-build/ninja.git', NINJA_COMMIT, ninja)
    run('cmake', '-S', ninja, '-B', ninja / 'out', '-DBUILD_TESTING=OFF', '-DCMAKE_BUILD_TYPE=Release')
    run('cmake', '--build', ninja / 'out', '--parallel', '4')
    gn = DIRECTORY / 'tools/gn'
    checkout('https://gn.googlesource.com/gn', skia['gnCommit'], gn)
    run('python3', 'build/gen.py', cwd=gn)
    run(ninja / 'out/ninja', '-C', 'out', '-j', '4', cwd=gn)
    run('python3', 'tools/components/build-image-codecs.py', '--target', 'native',
        '--archives', DIRECTORY / 'archives', '--directory', DIRECTORY / 'codecs', '--jobs', '4')
    run('python3', 'tools/components/build-harfbuzz.py', '--target', 'native',
        '--directory', DIRECTORY / 'harfbuzz', '--archiver', '/usr/bin/llvm-ar')
    run('python3', 'tools/components/build-skia.py', '--target', 'native',
        '--directory', DIRECTORY / 'skia', '--gn', gn / 'out/gn',
        '--ninja', ninja / 'out/ninja', '--archiver', '/usr/bin/llvm-ar',
        '--image-codecs', DIRECTORY / 'codecs', '--jobs', '4')


if __name__ == '__main__':
    main()
