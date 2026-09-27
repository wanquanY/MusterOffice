"""Build/verify an offline development plugin without installing or publishing it.

One canonical skill and portable manifest; Codex compatibility is a projection.
Native binaries are supplied explicitly, never downloaded or built implicitly.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import stat
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[2]
NAME = 'musteroffice'
SKILL = 'musteroffice-presentations'


def digest(path):
    result = hashlib.sha256()
    with Path(path).open('rb') as source:
        while block := source.read(1 << 20):
            result.update(block)
    return result.hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x', encoding='utf-8') as out:
        json.dump(value, out, indent=2, ensure_ascii=False); out.write('\n')


def regular(path):
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError(f'expected a regular non-symlink file: {path.name}')


def binary_target(path):
    regular(path)
    with path.open('rb') as f:
        data = f.read(1 << 20)
    if data[:4] == b'\xcf\xfa\xed\xfe' and len(data) >= 32:
        arch = {0x1000007: 'x86_64', 0x100000c: 'aarch64'}.get(struct.unpack_from('<I', data, 4)[0])
        os_name = 'macos'
    elif data[:6] == b'\x7fELF\x02\x01' and len(data) >= 64:
        arch = {62: 'x86_64', 183: 'aarch64'}.get(struct.unpack_from('<H', data, 18)[0])
        os_name = 'linux'
    elif data[:2] == b'MZ' and len(data) >= 64:
        offset = struct.unpack_from('<I', data, 60)[0]
        if offset + 24 > len(data) or data[offset:offset+4] != b'PE\0\0':
            raise ValueError('invalid PE binary header')
        arch = {0x8664: 'x86_64', 0xaa64: 'aarch64'}.get(struct.unpack_from('<H', data, offset+4)[0])
        os_name = 'windows'
    else:
        raise ValueError('unsupported binary format; supply a native MCP/export worker')
    if arch is None:
        raise ValueError('unsupported binary architecture')
    return os_name, arch


def copy_file(source, target, executable=False):
    regular(source)
    before = digest(source)
    target.parent.mkdir(parents=True, exist_ok=True)
    with source.open('rb') as incoming, target.open('xb') as outgoing:
        shutil.copyfileobj(incoming, outgoing, 1 << 20)
    target.chmod(0o755 if executable else 0o644)
    if digest(target) != before or digest(source) != before:
        raise ValueError('source changed during packaging')


def inventory(root):
    files = {}
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            raise ValueError('package symlinks are not supported')
        if path.is_dir():
            continue
        regular(path)
        name = path.relative_to(root).as_posix()
        if name == 'bundle-manifest.json':
            continue
        files[name] = dict(sha256=digest(path), byteLength=path.stat().st_size,
                           executable=bool(path.stat().st_mode & 0o111))
    return files


def verify(root):
    regular(root / 'bundle-manifest.json')
    manifest = json.loads((root / 'bundle-manifest.json').read_text())
    if manifest['format'] != 'musteroffice.agent-package/1-draft' or manifest['releaseCleared'] is not False:
        raise ValueError('unsupported development package manifest')
    if inventory(root) != manifest['files']:
        raise ValueError('package file inventory does not match')
    return manifest


def build(output, mcp, worker, source=ROOT):
    output, mcp, worker = output.absolute(), mcp.absolute(), worker.absolute()
    if output.name != NAME:
        raise ValueError('output directory name must be musteroffice')
    if any(output.resolve().is_relative_to((source / name).resolve()) for name in ('integrations', 'components')):
        raise ValueError('output must be outside package source material')
    target = binary_target(mcp)
    if binary_target(worker) != target:
        raise ValueError('MCP and export worker platform/architecture mismatch')
    portable = json.loads((source / 'integrations/plugin.json').read_text())
    if portable['name'] != NAME:
        raise ValueError('plugin name mismatch')
    output.parent.mkdir(parents=True, exist_ok=True)
    output.mkdir(mode=0o700)  # Exclusive; never rewrite an existing package.
    try:
        suffix = '.exe' if target[0] == 'windows' else ''
        copy_file(mcp, output / ('bin/mo-mcp' + suffix), True)
        copy_file(worker, output / ('bin/mo-export-worker' + suffix), True)
        skill = source / 'integrations/skills' / SKILL
        for path in sorted(skill.rglob('*')):
            if path.is_symlink():
                raise ValueError('skill cannot link outside its package')
            if path.is_file():
                copy_file(path, output / 'skills' / SKILL / path.relative_to(skill))
        if not (output / 'skills' / SKILL / 'SKILL.md').is_file():
            raise ValueError('canonical skill missing')
        copy_file(source / 'integrations/package-README.md', output / 'README.md')
        # Preserve recorded component licenses/notices; do not represent this
        # development inventory as completed transitive distribution clearance.
        for path in sorted((source / 'components').rglob('*')):
            if path.is_file() and any(word in path.name.lower() for word in ('license', 'notice', 'copyright', 'copying')):
                copy_file(path, output / 'notices' / path.relative_to(source / 'components'))
        write(output / 'plugin.json', portable)
        server = dict(type='stdio', command='./bin/mo-mcp' + suffix,
                      args=['--package', '${PLUGIN_ROOT}/runtime.json', '${PLUGIN_DATA}/caller-files.json'])
        write(output / 'mcp.json', {'$schema': 'https://agent-plugins.org/schemas/1.0.0/mcp.schema.json',
                                   'mcpServers': {NAME: server}})
        # Legacy clients do not apply the portable command path rule. The fixed
        # working directory binds the relative executable without a shell.
        legacy_server = dict(server, cwd='${PLUGIN_ROOT}')
        write(output / '.mcp.json', {'mcpServers': {NAME: legacy_server}})
        legacy = {k: portable[k] for k in ('name', 'version', 'description', 'author', 'keywords')}
        legacy.update(skills='./skills/', mcpServers='./.mcp.json',
                      interface=portable['extensions']['com.openai']['interface'])
        write(output / '.codex-plugin/plugin.json', legacy)
        write(output / 'runtime.json', dict(format='musteroffice.mcp-package/1-draft',
              targetOs=target[0], targetArch=target[1], workerSha256=digest(output / ('bin/mo-export-worker' + suffix))))
        files = inventory(output)
        write(output / 'bundle-manifest.json', dict(format='musteroffice.agent-package/1-draft',
              version=portable['version'], targetOs=target[0], targetArch=target[1], releaseCleared=False,
              scope='Offline development assembly. Host owns files, authorization, retention and publication.',
              files=files, skillSha256=digest(output / 'skills' / SKILL / 'SKILL.md')))
        return verify(output)
    except BaseException:
        shutil.rmtree(output)  # Only the directory exclusively created above.
        raise


def archive(root, destination):
    if destination.resolve().is_relative_to(root.resolve()):
        raise ValueError('archive must be outside the package it contains')
    verify(root)
    if destination.exists():
        raise FileExistsError('preserve previous package archive')
    with destination.open('xb') as output, zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED) as package:
        for path in sorted(root.rglob('*')):
            if not path.is_file():
                continue
            info = zipfile.ZipInfo(path.relative_to(root).as_posix(), (1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | (0o755 if path.stat().st_mode & 0o111 else 0o644)) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            with path.open('rb') as incoming, package.open(info, 'w', force_zip64=True) as outgoing:
                shutil.copyfileobj(incoming, outgoing, 1 << 20)
    verify(root)
    return digest(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--mcp', type=Path)
    parser.add_argument('--worker', type=Path)
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    if args.verify:
        result = verify(args.output)
    else:
        if not args.mcp or not args.worker:
            parser.error('--mcp and --worker are required for build')
        result = build(args.output, args.mcp, args.worker)
    summary = dict(files=len(result['files']), targetOs=result['targetOs'], targetArch=result['targetArch'], releaseCleared=False)
    if args.archive:
        summary['archiveSha256'] = archive(args.output, args.archive)
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
