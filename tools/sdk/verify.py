"""Verify a previously pinned local SDK directory, without executing its code."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re


def verify(root, expected):
    root = root.resolve()
    manifest_path = root / 'sdk-manifest.json'
    if manifest_path.is_symlink() or not manifest_path.is_file():
        raise ValueError('invalid SDK manifest file')
    with manifest_path.open('rb') as stream:
        raw = stream.read(4 * 1024 * 1024 + 1)
    if len(raw) > 4 * 1024 * 1024:
        raise ValueError('SDK manifest byte limit')
    if not re.fullmatch('[0-9a-f]{64}', expected) or hashlib.sha256(raw).hexdigest() != expected:
        raise ValueError('SDK manifest differs from the externally pinned SHA-256')
    manifest = json.loads(raw)
    if manifest['format'] != 'musteroffice.rust-embedded-sdk/1-draft':
        raise ValueError('unknown SDK format')
    if not isinstance(manifest['files'], list) or not 0 < len(manifest['files']) < 8192:
        raise ValueError('invalid SDK file count')
    names = set()
    folded = set()
    total = 0
    for item in manifest['files']:
        path = PurePosixPath(item['path'])
        if not re.fullmatch(r'[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*', item['path']) or path.is_absolute() or '..' in path.parts or str(path) != item['path'] or not path.parts:
            raise ValueError('invalid SDK path')
        if item['path'] == 'sdk-manifest.json' or item['path'].lower() in folded:
            raise ValueError('duplicate SDK file')
        if type(item['byteLength']) is not int or not 0 <= item['byteLength'] <= 64 * 1024 * 1024 or not re.fullmatch('[0-9a-f]{64}', item['sha256']):
            raise ValueError('invalid SDK file identity')
        total += item['byteLength']
        if total > 64 * 1024 * 1024:
            raise ValueError('SDK byte limit')
        names.add(item['path'])
        folded.add(item['path'].lower())
        target = root.joinpath(*path.parts)
        for parent in [target, *target.parents]:
            if parent == root:
                break
            if parent.is_symlink():
                raise ValueError('SDK symlink is not allowed')
        if not target.is_file() or target.stat().st_size != item['byteLength']:
            raise ValueError('SDK file type or length differs')
        length = 0
        digest = hashlib.sha256()
        with target.open('rb') as stream:
            while data := stream.read(65536):
                length += len(data)
                if length > item['byteLength']:
                    raise ValueError('SDK file grew beyond its identity')
                digest.update(data)
        if length != item['byteLength'] or digest.hexdigest() != item['sha256']:
            raise ValueError('SDK file bytes differ from the pinned manifest')
    actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() or p.is_symlink()}
    if actual != names | {'sdk-manifest.json'}:
        raise ValueError('SDK contains missing or additional files')
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--sha256', required=True)
    args = parser.parse_args()
    manifest = verify(args.directory, args.sha256)
    print(json.dumps(dict(files=len(manifest['files']), libraries=len(manifest['libraries']))))
