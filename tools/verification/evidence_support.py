"""Shared evidence readers; historical stage scripts and reports stay immutable.

These helpers inspect saved observations. They do not execute tests, manufacture
missing exit codes, or turn a local check into a product acceptance result.
"""
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import unquote


def cargo_error(log):
    # Rust module/test names can contain `::error::`. Only Cargo/rustc
    # diagnostic lines are errors, including rustc's numbered diagnostics.
    return re.search(r'^\s*error(?:\[[^\]\r\n]+\])?:', log, re.M) is not None


def read_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def file_entry(path, base=Path('.')):
    path, base = Path(path), Path(base)
    digest, count = hashlib.sha256(), 0
    with path.open('rb') as stream:
        while block := stream.read(1 << 20):
            digest.update(block)
            count += len(block)
    return dict(path=path.relative_to(base).as_posix(), byteLength=count, sha256=digest.hexdigest())


def cargo_tests(path):
    log = Path(path).read_text(encoding='utf-8')
    if re.search(r'^test .+ \.\.\. FAILED$|^test result: (?!ok\.)', log, re.M) or cargo_error(log):
        raise ValueError(f'failed test log: {path}')
    names = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
    summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
    if not names or not summaries or sum(int(p) for p, _, _ in summaries) != len(names):
        raise ValueError(f'incomplete test observations: {path}')
    if any(int(f) for _, f, _ in summaries):
        raise ValueError(f'failed test summary: {path}')
    return dict(passed=len(names), ignored=sum(int(i) for _, _, i in summaries), names=names)


def cargo_check(path, strict=False):
    log = Path(path).read_text(encoding='utf-8')
    if 'Finished `dev` profile' not in log or cargo_error(log) or (strict and 'warning:' in log):
        raise ValueError(f'failed or incomplete check: {path}')


def recorded_command(stage, name):
    """Require a successful command, unchanged inputs and its exact saved log."""
    stage = Path(stage)
    record = read_json(stage / (name + '.json'))
    if record['exitCode'] != 0 or record['sourceBefore'] != record['sourceAfter']:
        raise ValueError(f'failed command or changed inputs: {name}')
    if file_entry(stage / (name + '.log'), stage)['sha256'] != record['logSha256']:
        raise ValueError(f'command log differs: {name}')
    return record


def local_links(sources, pending=()):
    pending = {Path(p).resolve() for p in pending}
    count = 0
    for name in sources:
        path = Path(name)
        if path.suffix != '.md':
            continue
        for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', path.read_text(encoding='utf-8')):
            if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
                continue
            target = unquote(target.split('#')[0].split('?')[0])
            if target:
                resolved = (path.parent / target).resolve()
                if not resolved.exists() and resolved not in pending:
                    raise ValueError(f'missing local link in {name}: {target}')
                count += 1
    return count


def write_report(path, report, check=False):
    text = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
    if check:
        if Path(path).read_text(encoding='utf-8') != text:
            raise ValueError('saved report does not match current observations')
    else:
        with Path(path).open('x', encoding='utf-8') as stream:
            stream.write(text)
