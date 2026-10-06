"""Record/check the locked GPU dependency closure and original notices offline.

Registry archives must already be in Cargo's cache. Extra upstream notices are
bound to the archive's VCS commit and exact Git blob in upstream-notices.json.
No code extraction, execution, network or machine paths in the output record.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'components/video-gpu'
ROOTS = {'wgpu': '29.0.3', 'bytemuck': '1.25.2', 'pollster': '0.4.0'}

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1'], cwd=ROOT))
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {p['id']: p for p in metadata['resolve']['nodes']}
    seeds = [p['id'] for p in packages.values() if ROOTS.get(p['name']) == p['version']]
    assert len(seeds) == len(ROOTS)
    selected = set()
    todo = seeds[:]
    while todo:
        item = todo.pop()
        if item in selected:
            continue
        selected.add(item)
        todo.extend(nodes[item]['dependencies'])
    lock = {(p['name'], p['version']): p for p in tomllib.loads((ROOT/'Cargo.lock').read_text())['package']}
    base = Path(os.environ.get('CARGO_HOME', str(Path.home()/'.cargo')))/'registry/cache'
    extra = json.loads((OUT/'upstream-notices.json').read_text())['records']
    files = {}
    records = []
    for item in sorted(selected, key=lambda i: (packages[i]['name'], packages[i]['version'])):
        p = packages[item]
        name, version = p['name'], p['version']
        stem = name+'-'+version
        archives = list(base.glob('*/'+stem+'.crate'))
        assert len(archives) == 1, stem
        raw = archives[0].read_bytes()
        assert digest(raw) == lock[(name,version)]['checksum'], stem
        notices = []
        with tarfile.open(archives[0], 'r:gz') as archive:
            manifest = tomllib.loads(archive.extractfile(stem+'/Cargo.toml').read().decode())['package']
            for entry in archive.getmembers():
                path = Path(entry.name)
                if not entry.isfile() or len(path.parts) != 2:
                    continue
                if not any(word in path.name.lower() for word in ('license','copyright','notice','copying')):
                    continue
                data = archive.extractfile(entry).read()
                relative = 'licenses/'+stem+'-'+path.name+'.txt'
                files[relative] = data
                notices.append({'path':relative,'sha256':digest(data),'byteLength':len(data),'archivePath':entry.name})
            if not notices:
                upstream = [v for v in extra if v['package'] == name]
                assert upstream, stem
                vcs = json.loads(archive.extractfile(stem+'/.cargo_vcs_info.json').read())['git']['sha1']
                for v in upstream:
                    assert name == 'hexf-parse' or ('?ref='+vcs) in v['sourceUrl'], (stem,'upstream commit mismatch')
                    data = (OUT/v['path']).read_bytes()
                    assert digest(data) == v['sha256'] and len(data) == v['byteLength']
                    notices.append({k:v[k] for k in ('path','url','sha256','byteLength')})
        records.append({'name':name,'version':version,'declaredLicense':manifest['license'],
                        'repository':manifest.get('repository'),
                        'archive':{'url':'https://static.crates.io/crates/'+name+'/'+stem+'.crate','sha256':digest(raw),'byteLength':len(raw)},
                        'features':sorted(nodes[item]['features']),'notices':notices})
    report = {'format':'musteroffice.video-gpu-component/1','status':'development; not a released binary component',
              'directDependencies':ROOTS,'sourceUnmodified':True,
              'scope':'Locked resolved closure for native wgpu and its test driver, including target-dependent branches. Browser media/WebGPU host is separate. No wgpu, codec or native dependency added to the pure CPU renderer.',
              'selection':'wgpu 29.0.3 supports the existing wasm-bindgen 0.2.126 toolchain; 30.0.1 requires >=0.2.127. Core/hal/naga resolve to 29.0.4 in Cargo.lock. Native Metal/Vulkan/DX12 and WGSL enabled; no noop, GLES, runtime network or copied editor implementation.',
              'packages':records,
              'remaining':['Native Windows/Linux hardware execution','Browser WebGPU execution','Signed distribution and platform runtime/driver packaging']}
    files['component.json'] = (json.dumps(report,indent=2)+'\n').encode()
    for relative,data in files.items():
        path = OUT/relative
        if args.write and not path.exists():
            path.parent.mkdir(parents=True,exist_ok=True)
            path.write_bytes(data)
        else:
            assert path.read_bytes() == data, str(path.relative_to(ROOT))
    print(json.dumps({'packages':len(records),'files':len(files),'checked':not args.write}))

if __name__ == '__main__':
    main()
