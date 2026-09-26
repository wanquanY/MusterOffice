"""Paired isolated ownership measurements; records limits and actual environment."""
import hashlib,json,platform,subprocess
from pathlib import Path
assert platform.system()=='Darwin','RSS units in this measurement profile are macOS bytes'
root=Path('.codex-work/paint-admission');exe=Path('target/release/examples/gradient_storage')
def entry(p):
 b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
records=[]
for i in range(7):
 for mode in (['shared','copied-array-model'] if i%2==0 else ['copied-array-model','shared']):
  proc=subprocess.run(['/usr/bin/time','-l',str(exe),mode],text=True,capture_output=True,check=True)
  info=json.loads(proc.stdout);rss=[int(s.split()[0]) for s in proc.stderr.splitlines() if 'maximum resident set size' in s];assert len(rss)==1
  assert info['retainedStopBuffers']==(1 if mode=='shared' else 64)
  assert (info['instances'],info['stopsPerInstance'],info['stopBytes'])==(64,4096,40)
  path=root/(f'storage-{i}-{mode}.txt');path.write_text(proc.stdout+proc.stderr)
  records.append(dict(repetition=i,maximumResidentBytes=rss[0],**info,log=entry(path)))
assert len({r['checksum'] for r in records})==1
result=dict(format='musteroffice.paint-storage-measurement/1',executable=entry(exe),source=entry('crates/mo-raster/examples/gradient_storage.rs'),platform=dict(system=platform.system(),release=platform.release(),machine=platform.machine(),cpu=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),memoryBytes=int(subprocess.check_output(['sysctl','-n','hw.memsize'],text=True)),rustc=subprocess.check_output(['rustc','--version'],text=True).strip()),build='cargo build --release -p mo-raster --example gradient_storage --locked; workspace release thin LTO, codegen-units=1, overflow checks on',scope='Fresh process, 64 evaluated paints, 4096 stops each; zero fonts/images. Shared is actual Brush::rebased; control is independent Vec<GradientStop> clones modeling the former stop copy mechanism, not a historical full-engine build. Cache state uncontrolled, process page cache warm after first run. Stop payload excludes objects, allocator headers, baseline arrays and compiler/raster buffers. RSS is whole process; time is construction only. No product, installer or full-page performance claim.',cases=records)
(root/'storage.json').write_text(json.dumps(result,indent=2)+'\n')
for mode in ['shared','copied-array-model']:
 rows=[r for r in records if r['mode']==mode]
 print(json.dumps(dict(mode=mode,stopPayloadBytes=rows[0]['retainedStopPayloadBytes'],rssBytes=sorted(r['maximumResidentBytes'] for r in rows),constructionNanos=sorted(r['constructionNanos'] for r in rows))))
