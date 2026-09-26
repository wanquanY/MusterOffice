"""Execute actual disk delivery and ignored worker integration tests explicitly."""
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import subprocess

root = Path('.codex-work/delivery-pipeline')
worker = Path('target/debug/mo-raster-worker').resolve()


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


identity = entry('target/debug/mo-raster-worker')
env = dict(os.environ, CARGO_BUILD_JOBS='2',
           MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()),
           MO_DELIVERY_WORKER=str(worker), MO_DELIVERY_WORKER_SHA256=identity['sha256'])
commands = [
    ('candidate', ['target/debug/examples/delivery', str(root/'final'),
                   'fixtures/presentations/delivery/input.json',
                   'fixtures/presentations/native-export/resources.bin', 'fixtures/fonts/owned.ttf',
                   str(worker), identity['sha256']]),
    ('native-integration', ['cargo', 'test', '-p', 'mo-native-render', '--test', 'delivery',
                            '--locked', '--offline', '--', '--ignored']),
    ('independent-reference', ['python3', 'tools/verification/delivery-reference.py', str(root/'final')]),
]
records = []
for name, argv in commands:
    log = root/f'{name}-final.log'
    with log.open('x') as stream:
        run = subprocess.run(argv, env=env, stdout=stream, stderr=subprocess.STDOUT)
    records.append(dict(name=name, argv=argv, exitCode=run.returncode, log=entry(log)))
    print(json.dumps(records[-1]), flush=True)
    assert run.returncode == 0, name
assert entry(identity['path']) == identity
report = dict(format='musteroffice.delivery-native/1', worker=identity,
              example=entry('target/debug/examples/delivery'), calls=records,
              verifierRuntime=dict(python=platform.python_version(), platform=platform.platform(),
                                   packages={name:importlib.metadata.version(name) for name in ['Pillow', 'lxml', 'python-pptx', 'jsonschema', 'referencing']}),
              scope='Actual private output calculation; no host publication or target-application acceptance.')
with (root/'native.json').open('x') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
