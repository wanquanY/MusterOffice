"""Assemble a local WASM playback SDK from explicit verified build products.

No installation, dependency download, package publication or license decision.
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

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        while b:=f.read(1<<20):h.update(b)
    return h.hexdigest()


def copy(source, target):
    if source.is_symlink() or not source.is_file():raise ValueError('regular input required')
    expected=sha(source);target.parent.mkdir(parents=True,exist_ok=True)
    with source.open('rb') as incoming,target.open('xb') as outgoing:shutil.copyfileobj(incoming,outgoing)
    target.chmod(0o644)
    if sha(source)!=expected or sha(target)!=expected:raise ValueError('input changed while copying')


def write(path, value):
    with path.open('x') as f:json.dump(value,f,indent=2);f.write('\n')


def inventory(root):
    entries=[]
    for p in sorted(root.rglob('*')):
        if p.is_symlink():raise ValueError('symlinks not allowed')
        if p.is_dir():continue
        if not p.is_file():raise ValueError('non-regular package file')
        if p==root/'bundle-manifest.json':continue
        entries.append(dict(path=p.relative_to(root).as_posix(),sha256=sha(p),byteLength=p.stat().st_size))
    return entries


def verify(root, expected):
    path=root/'bundle-manifest.json'
    if path.is_symlink() or not path.is_file() or path.stat().st_size>1024*1024 or sha(path)!=expected:
        raise ValueError('manifest differs from external pin')
    m=json.loads(path.read_text())
    if m['format']!='musteroffice.wasm-playback-sdk/1-draft' or m['releaseCleared'] is not False:
        raise ValueError('unsupported development SDK manifest')
    if m['files']!=inventory(root):raise ValueError('package inventory differs')
    return m


def build(output, wasm, raster, text):
    output=output.resolve()
    for source in [ROOT/'packages',ROOT/'components',ROOT/'tools',wasm.resolve(),raster.resolve(),text.resolve()]:
        if output.is_relative_to(source):raise ValueError('output cannot modify input material')
    output.mkdir(parents=True,exist_ok=False)
    subprocess.run(['pnpm','exec','tsc','--project','packages/playback-client/tsconfig.json',
                    '--outDir',str(output/'lib')],cwd=ROOT,check=True)
    for name in ['index.mjs','index.d.mts']:copy(ROOT/'packages/playback-client/bundle'/name,output/name)
    copy(ROOT/'packages/playback-client/README.md',output/'README.md')
    copy(ROOT/'tools/playback-sdk/example.mjs',output/'examples/node-worker.mjs')
    for name in ['dispatch.mjs','browser-worker.mjs','browser-runtime.mjs']:
        copy(ROOT/'tools/playback-sdk'/name,output/'examples'/name)
    for directory,names in [(wasm,['mo_wasm.js','mo_wasm.d.ts','mo_wasm_bg.wasm']),
                            (raster,['mo-skia.mjs','mo-skia.wasm']),(text,['mo-hb.mjs','mo-hb.wasm'])]:
        for name in names:copy(directory/name,output/'runtime'/name)
    write(output/'package.json',dict(name='@musteroffice/playback-wasm',version='0.1.0',private=True,type='module',
          exports={'.':{'types':'./index.d.mts','import':'./index.mjs'}}))
    notice_roots=['skia','harfbuzz','image-codec','unicode-bidi','rust-numeric','drawingml-presets']
    for name in notice_roots:
        for p in sorted((ROOT/'components'/name).rglob('*')):
            if p.is_file() and (any(tag in p.name.upper() for tag in ['LICENSE','COPYRIGHT','NOTICE','AUTHORS'])
                                or p.name in {'lock.json','component.json'}):
                copy(p,output/'notices'/p.relative_to(ROOT/'components'))
    files=inventory(output)
    write(output/'bundle-manifest.json',dict(format='musteroffice.wasm-playback-sdk/1-draft',releaseCleared=False,
          scope='Local development; caller-owned Worker, explicit code/assets, existing partial profiles.',files=files))
    verify(output,sha(output/'bundle-manifest.json'))
    return dict(files=len(files),manifestSha256=sha(output/'bundle-manifest.json'))


def archive(root, target):
    with target.open('xb') as file,gzip.GzipFile(fileobj=file,mode='wb',filename='',mtime=0) as compressed:
        with tarfile.open(fileobj=compressed,mode='w',format=tarfile.PAX_FORMAT) as tar:
            for p in sorted(root.rglob('*')):
                if not p.is_file():continue
                data=p.read_bytes();entry=tarfile.TarInfo(p.relative_to(root).as_posix())
                entry.size=len(data);entry.mode=0o644;entry.mtime=0;tar.addfile(entry,io.BytesIO(data))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',type=Path);p.add_argument('--wasm-dir',type=Path)
    p.add_argument('--raster-dir',type=Path);p.add_argument('--text-dir',type=Path)
    p.add_argument('--archive',type=Path);p.add_argument('--verify',type=Path);p.add_argument('--sha256')
    a=p.parse_args()
    if a.verify:
        print(json.dumps(dict(files=len(verify(a.verify,a.sha256)['files']))));return
    if not all([a.output,a.wasm_dir,a.raster_dir,a.text_dir]):p.error('output and all explicit module directories are required')
    result=build(a.output,a.wasm_dir,a.raster_dir,a.text_dir)
    if a.archive:archive(a.output,a.archive)
    print(json.dumps(result))


if __name__=='__main__':main()
