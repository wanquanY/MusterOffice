"""Record a product command against explicit source/material inventories.

This runner records observations only; a successful command is not a product
acceptance verdict. Reports are exclusive and source changes fail the run.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def snapshot(root, sources):
    files = set()
    for source in sources:
        path = root / source
        if not path.exists():
            raise ValueError(f'missing source or material: {source}')
        if path.is_file():
            files.add(path)
        else:
            files.update(p for p in path.rglob('*') if p.is_file()
                         and not {'.git', 'target', '__pycache__'}.intersection(p.parts))
    result = {}
    for path in sorted(files):
        digest = hashlib.sha256()
        with path.open('rb') as stream:
            while block := stream.read(1 << 20):
                digest.update(block)
        result[path.relative_to(root).as_posix()] = digest.hexdigest()
    return result


def run(root, stage, name, sources, command):
    if not name or Path(name).name != name or name in ('.', '..'):
        raise ValueError('name must be one path component')
    if not sources or not command:
        raise ValueError('explicit source inventory and command are required')
    stage.mkdir(parents=True, exist_ok=True)
    log, report = stage / (name + '.log'), stage / (name + '.json')
    if log.exists() or report.exists():
        raise FileExistsError('preserve previous command evidence')
    before = snapshot(root, sources)
    start = time.monotonic()
    # Exclusive creation also protects against a concurrent run of this name.
    with log.open('x', encoding='utf-8') as output:
        try:
            code = subprocess.run(command, cwd=root, stdout=output,
                                  stderr=subprocess.STDOUT).returncode
        except OSError as error:
            output.write(f'command launch failed: {error}\n')
            code = 127
    after = snapshot(root, sources)
    value = dict(args=command, exitCode=code,
                 elapsedSeconds=time.monotonic() - start,
                 sourceInventory=sources, sourceBefore=before, sourceAfter=after,
                 sourceUnchanged=before == after,
                 logSha256=hashlib.sha256(log.read_bytes()).hexdigest())
    with report.open('x', encoding='utf-8') as output:
        json.dump(value, output, indent=2)
        output.write('\n')
    print(json.dumps({key: value[key] for key in
                      ('exitCode', 'elapsedSeconds', 'sourceUnchanged', 'logSha256')}))
    print('\n'.join(log.read_text(encoding='utf-8', errors='replace').splitlines()[-25:]))
    return code if code else 0 if before == after else 2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--stage', type=Path, required=True)
    parser.add_argument('--name', required=True)
    parser.add_argument('--source', action='append', required=True)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    return run(args.root.resolve(), args.stage.resolve(), args.name, args.source, command)


if __name__ == '__main__':
    raise SystemExit(main())
