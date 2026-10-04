#!/usr/bin/env python3
"""Run the README's real CLI export, import inspection and composition examples."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / '.codex-work/ci-smoke'


def command(name, *args):
    data = subprocess.check_output([str(ROOT / 'target/debug/mo-cli'), *map(str, args)], cwd=ROOT)
    value = json.loads(data)
    if value.get('status') == 'failed' or value.get('outcome') == 'failed' or 'error' in value:
        raise RuntimeError(f'{name} failed: {value}')
    (OUTPUT / (name + '.json')).write_bytes(data)
    return value


def main():
    OUTPUT.mkdir(parents=True, exist_ok=False)
    command('export', 'pptx-export', 'fixtures/presentations/native-export/request.json',
            'fixtures/presentations/native-export/resources.bin', OUTPUT / 'hello.pptx')
    command('inspect-hello', 'pptx-inspect', OUTPUT / 'hello.pptx')
    for name in ['business-blueprint', 'editorial-perspective']:
        command('inspect-' + name, 'pptx-inspect', f'examples/templates/{name}/{name}.pptx')
    (OUTPUT / 'temporary').mkdir()
    (OUTPUT / 'inputs.json').write_text('[]\n')
    command('compose', 'compute', 'fixtures/presentations/compose/invocation.json',
            OUTPUT / 'inputs.json', OUTPUT / 'temporary', OUTPUT / 'composed')
    result = json.loads((OUTPUT / 'composed/result.json').read_text())
    if result['result']['kind'] != 'mutated':
        raise RuntimeError('Composition did not return a document snapshot')
    command('schema', 'compute-schema', 'computation-invocation')
    print('README CLI export, template inspection and composition passed')


if __name__ == '__main__':
    main()
