"""Execute the entire owned decode corpus through the instrumented C++ closure."""
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess

root = Path('.codex-work/image-codec')


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


build_path = root / 'component/native-asan-build.json'
build = json.loads(build_path.read_text())
assert build['sanitizers'] and build['imageCodecs']['sanitizers']
for record in build['artifacts'] + build['componentSources']:
    assert entry(record['path']) == record
suffixes = ['libmo_skia_adapter_asan.a', 'libskia.a', 'libpng16.a', 'libjpeg.a', 'libz.a']
libraries = [next(r['path'] for r in build['artifacts'] if r['path'].endswith('/'+s)) for s in suffixes]
probe = root / 'decode-probe-asan'
command = ['clang++', '-std=c++20', '-O2', '-g1', '-fno-omit-frame-pointer',
           '-fsanitize=address,undefined', '-Icomponents/image-codec', '-Icomponents/skia',
           'tools/verification/image-codec-probe.cpp', *libraries, '-Wl,-dead_strip', '-o', str(probe)]
subprocess.run(command, check=True)
cases = json.loads((root/'cases/manifest.json').read_text())
payload = b''
for case in cases:
    data = Path(case['path']).read_bytes()
    payload += struct.pack('<I',len(data)) + data
env = os.environ.copy()
env['UBSAN_OPTIONS'] = 'halt_on_error=1:print_stacktrace=1'
env['ASAN_OPTIONS'] = 'halt_on_error=1'
result = subprocess.run([probe], input=payload, capture_output=True, env=env, timeout=120, check=True)
assert not result.stderr, result.stderr.decode()
offset = 0
for case in cases:
    status, *words = struct.unpack_from('<10I', result.stdout, offset)
    offset += 40
    pixels = result.stdout[offset:offset+words[8]]
    offset += words[8]
    if 'expected' in case:
        assert status == 0 and words[0:2] == [case['width'],case['height']], case['name']
        assert pixels == (root/'runtime'/f"{case['name']}.rgba").read_bytes(), case['name']
    else:
        assert status == {'INPUT_INVALID':1,'LIMIT_EXCEEDED':3,'UNSUPPORTED':5}[case['code']], case['name']
        assert words == [0]*9 and not pixels
assert offset == len(result.stdout)
report = dict(format='musteroffice.image-codec-sanitizers/1', cases=len(cases),
              addressSanitizer=True, undefinedBehaviorSanitizer=True,
              leakSanitizer=False, noDiagnostics=True, command=command, build=entry(build_path),
              probe=entry(probe), input=entry(root/'cases/manifest.json'),
              outputSha256=hashlib.sha256(result.stdout).hexdigest())
(root/'sanitizers.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(cases=len(cases),addressSanitizer=True,undefinedBehaviorSanitizer=True,noDiagnostics=True)))
