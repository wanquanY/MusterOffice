"""Prepare a reproducible local Rust SDK from the current owned build inputs.

This does not publish, install, fetch dependencies or choose a project license.
The receiving product pins sdk-manifest.json before consuming the directory.
"""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
ROOT_CRATE = 'mo-embedded-sdk'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def record(path, base):
    data = path.read_bytes()
    return dict(path=path.relative_to(base).as_posix(), byteLength=len(data), sha256=sha(data))


def toml_value(value):
    if isinstance(value, bool):
        return 'true' if value else 'false'
    if isinstance(value, (str, int)):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, list):
        return '[' + ', '.join(toml_value(v) for v in value) + ']'
    if isinstance(value, dict):
        return '{ ' + ', '.join(f'{json.dumps(k)} = {toml_value(v)}' for k, v in sorted(value.items())) + ' }'
    raise ValueError(f'unsupported TOML value type: {type(value).__name__}')


def toml_document(value):
    # Inline tables retain dotted workspace inheritance without a TOML writer
    # dependency. Every generated document is parsed again before it is saved.
    result = '\n'.join(f'{json.dumps(k)} = {toml_value(v)}' for k, v in sorted(value.items())) + '\n'
    assert tomllib.loads(result) == value
    return result


def production_dependencies(manifest):
    """Keep platform predicates intact; development edges cannot enter the SDK."""
    manifest.pop('dev-dependencies', None)
    dependencies = list(manifest.get('dependencies', {}).items())
    for selector, target in manifest.get('target', {}).items():
        if not isinstance(target, dict) or set(target) - {'dependencies', 'dev-dependencies'}:
            raise ValueError(f'unsupported SDK target table: {selector}')
        target.pop('dev-dependencies', None)
        dependencies.extend(target.get('dependencies', {}).items())
    return dependencies


def build(destination):
    destination = destination.resolve()
    destination.mkdir(parents=True, exist_ok=False)
    workspace = tomllib.loads((ROOT / 'Cargo.toml').read_text())
    dependency_table = workspace['workspace']['dependencies']
    selected = {}
    required_dependencies = set()
    pending = [ROOT_CRATE]
    while pending:
        name = pending.pop()
        if name in selected:
            continue
        declaration = dependency_table[name]
        assert isinstance(declaration, dict) and 'path' in declaration
        directory = ROOT / declaration['path']
        assert directory.resolve().is_relative_to(ROOT / 'crates')
        manifest = tomllib.loads((directory / 'Cargo.toml').read_text())
        assert manifest['package']['name'] == name
        # Distribution is the production library closure. Dev/test graphs are
        # verified in the source repository, not embedded into a host product.
        assert not any(k in manifest for k in ['build-dependencies', 'bin', 'example', 'test', 'bench'])
        assert 'build' not in manifest['package'] and not (directory / 'build.rs').exists()
        production = production_dependencies(manifest)
        manifest['package'].update(autobins=False, autoexamples=False, autotests=False, autobenches=False)
        # A product may vendor this SDK beneath its own Cargo workspace. Pin
        # each library to the SDK workspace so inherited dependencies cannot
        # silently resolve against that enclosing product's dependency table.
        manifest['package']['workspace'] = '../..'
        manifest.setdefault('lib', {}).update(test=False, doctest=False, bench=False)
        selected[name] = (directory, manifest)
        for key, dep in production:
            assert isinstance(dep, dict) and dep.get('workspace') is True, (name, key)
            required_dependencies.add(key)
            inherited = dependency_table[key]
            if isinstance(inherited, dict) and 'path' in inherited:
                pending.append(key)
    forbidden = {'mo-operation-service', 'mo-standard-host', 'mo-skia-sys', 'mo-harfbuzz-sys', 'mo-wasm', 'rusqlite', 'tokio'}
    assert not forbidden.intersection(selected.keys() | required_dependencies)
    generated_workspace = {
        'workspace': {
            'resolver': workspace['workspace']['resolver'],
            'members': [dependency_table[n]['path'] for n in sorted(selected)],
            'package': workspace['workspace']['package'],
            'dependencies': {k: dependency_table[k] for k in sorted(required_dependencies)},
            'lints': workspace['workspace']['lints'],
        },
        'profile': workspace['profile'],
    }
    inputs = set()

    def copy(path, relative=None):
        assert path.is_file() and not path.is_symlink(), path
        assert path.resolve().is_relative_to(ROOT), path
        inputs.add(path)
        target = destination / (relative or path.relative_to(ROOT))
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(path.read_bytes())

    for name, (directory, manifest) in sorted(selected.items()):
        for child in ['src', 'data']:
            for path in sorted((directory / child).rglob('*')):
                if path.is_file():
                    copy(path)
        inputs.add(directory / 'Cargo.toml')
        target = destination / directory.relative_to(ROOT) / 'Cargo.toml'
        target.write_text(toml_document(manifest))
    # Preset includes retain their exact source-relative paths and notices.
    for name in ['drawingml-presets', 'unicode-bidi', 'rust-numeric', 'rustix']:
        for path in sorted((ROOT / 'components' / name).rglob('*')):
            if path.is_file() and path.name != 'README.md':
                copy(path)
    for path in sorted((ROOT / 'tools/sdk/example').rglob('*')):
        if path.is_file():
            copy(path, Path('examples/native-export') / path.relative_to(ROOT / 'tools/sdk/example'))
    copy(ROOT / 'tools/sdk/README.md', Path('README.md'))
    copy(ROOT / 'tools/sdk/verify.py', Path('verify.py'))
    inputs |= {ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__).resolve()}
    (destination / 'Cargo.toml').write_text(toml_document(generated_workspace))
    shutil.copyfile(ROOT / 'Cargo.lock', destination / 'Cargo.lock')
    # Use the existing lock as the resolution seed; offline prohibits network
    # and the comparison below prohibits selecting new external versions.
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--offline', '--format-version=1'], cwd=destination))
    original_lock = tomllib.loads((ROOT / 'Cargo.lock').read_text())
    lock = tomllib.loads((destination / 'Cargo.lock').read_text())
    registry = {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                for p in original_lock['package'] if 'source' in p}
    actual_registry = {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                       for p in lock['package'] if 'source' in p}
    assert actual_registry <= registry, actual_registry - registry
    assert {p['name'] for p in metadata['packages'] if p['source'] is None} == set(selected)
    assert not forbidden.intersection(p['name'] for p in metadata['packages'])
    registry_packages = [dict(name=p['name'], version=p['version'], license=p['license'], source=p['source'])
                         for p in metadata['packages'] if p['source'] is not None]
    files = [record(p, destination) for p in sorted(destination.rglob('*')) if p.is_file()]
    manifest = dict(format='musteroffice.rust-embedded-sdk/1-draft',
                    version=workspace['workspace']['package']['version'], rootCrate=ROOT_CRATE,
                    rustVersion=workspace['workspace']['package']['rust-version'],
                    sourceFiles=[record(p, ROOT) for p in sorted(inputs)],
                    libraries=sorted(selected), registryPackages=registry_packages, files=files,
                    scope='Local development SDK build inputs. No publication or project license selection. No worker, fonts, database or product authority included.')
    (destination / 'sdk-manifest.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')
    return manifest


def archive(source, destination):
    with destination.open('xb') as output:
        with gzip.GzipFile(filename='', mode='wb', fileobj=output, mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode='w', format=tarfile.USTAR_FORMAT) as tar:
                for path in sorted(source.rglob('*')):
                    if not path.is_file():
                        continue
                    data = path.read_bytes()
                    info = tarfile.TarInfo(path.relative_to(source).as_posix())
                    info.size, info.mode, info.mtime = len(data), 0o644, 0
                    tar.addfile(info, io.BytesIO(data))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--archive', type=Path)
    args = parser.parse_args()
    result = build(args.output)
    if args.archive:
        archive(args.output, args.archive)
    print(json.dumps(dict(libraries=len(result['libraries']), registryPackages=len(result['registryPackages']),
                          files=len(result['files']), manifest=record(args.output / 'sdk-manifest.json', args.output))))
