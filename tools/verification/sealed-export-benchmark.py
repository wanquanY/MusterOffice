"""Local debug-host resource/export comparison; never an installer or QA gate."""
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import re
import statistics
import struct
import subprocess
import zlib

root=Path('.codex-work/sealed-export/benchmark');root.mkdir()
entry=lambda p:dict(path=str(p),byteLength=Path(p).stat().st_size,sha256=hashlib.sha256(Path(p).read_bytes()).hexdigest())
parent=json.loads(Path('docs/reviews/evidence/2026-09-26-resource-host-verification.json').read_text())
baseline=Path('.codex-work/sealed-export/baseline/mo-cli')
assert entry(baseline)['sha256']==parent['unchangedKernelArtifacts']['nativeCli']['sha256']
fixture=Path('fixtures/presentations/native-export/request.json')
request=json.loads(fixture.read_text())
def chunk(kind,body):
    return struct.pack('>I',len(body))+kind+body+struct.pack('>I',zlib.crc32(kind+body))
width=height=2048
rng=random.Random(20260926)
raw=b''.join(b'\0'+rng.randbytes(width*4) for _ in range(height))
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw,0))+chunk(b'IEND',b'')
assert zlib.decompress(png[41:-16])==raw
image=root/'rgba2048.png';image.write_bytes(png)
request['document']['resources']['resource:checker']['sha256']=hashlib.sha256(png).hexdigest()
request['resourceBindings'][0]['byteLength']=str(len(png))
large=root/'large.json';large.write_text(json.dumps(request,ensure_ascii=False)+'\n')
del png,raw,request
programs={'previous':baseline,'streamed':Path('target/debug/mo-cli')}
records=[]
for case,request,resources in [('fixture',fixture,Path('fixtures/presentations/native-export/resources.bin')),('large',large,image)]:
    expected=None
    for iteration in range(4):
        order=['previous','streamed'] if iteration%2==0 else ['streamed','previous']
        for name in order:
            target=root/f'{case}-{iteration}-{name}.pptx'
            run=subprocess.run(['/usr/bin/time','-lp',str(programs[name]),'pptx-export',str(request),str(resources),str(target)],capture_output=True,text=True,timeout=120)
            log=root/f'{case}-{iteration}-{name}.time';log.write_text(run.stderr)
            assert run.returncode==0,run.stderr
            receipt=json.loads(run.stdout);result=entry(target)
            assert receipt['status']=='inspected' and receipt['report']['sha256']==result['sha256']
            if expected is None:expected=result['sha256']
            assert result['sha256']==expected
            rss=re.search(r'^\s*(\d+)\s+maximum resident set size\s*$',run.stderr,re.M)
            wall=re.search(r'^real\s+([\d.]+)\s*$',run.stderr,re.M)
            assert rss and wall,run.stderr
            records.append(dict(case=case,implementation=name,iteration=iteration,warmup=iteration==0,peakRssBytes=int(rss[1]),wallSeconds=float(wall[1]),output=result,measurement=entry(log)))
            print(json.dumps({k:records[-1][k] for k in ['case','implementation','iteration','peakRssBytes','wallSeconds']}),flush=True)
assert not any(p.name.startswith('mo-spool-') for p in root.iterdir())
summary={}
for case in ['fixture','large']:
    summary[case]={}
    for impl in programs:
        trials=[r for r in records if r['case']==case and r['implementation']==impl and not r['warmup']]
        summary[case][impl]={metric:statistics.median(r[metric] for r in trials) for metric in ['peakRssBytes','wallSeconds']}
    summary[case]['peakRssReductionPercent']=100*(1-summary[case]['streamed']['peakRssBytes']/summary[case]['previous']['peakRssBytes'])
report=dict(format='musteroffice.sealed-export-debug-benchmark/1',environment=dict(platform=platform.platform(),architecture=platform.machine(),python=platform.python_version(),cpu=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),memoryBytes=int(subprocess.check_output(['sysctl','-n','hw.memsize'],text=True)),cargoProfile='debug, same Rust toolchain, unstripped',cache='OS caches not flushed; one excluded warmup per workload and implementation, three sequential alternating measured trials; no compiler running',measurement='macOS /usr/bin/time -lp, peak RSS bytes and wall seconds including process startup, stream, validation and local link publication',font='Explicit family references from fixture; no font bytes loaded, no shaping/rendering'),programs={k:entry(v) for k,v in programs.items()},inputs=[entry(fixture),entry('fixtures/presentations/native-export/resources.bin'),entry(large),entry(image)],records=records,summary=summary,limitations=['Two local debug workloads; not release throughput, sustained RSS, complete slides, rendering/playback, full resource budget or Musterwork installation estimate.', 'Large test is one generated 2048x2048 RGBA PNG, not a user deck. Authored file bytes agree with previous implementation.', 'Package XML/metadata remain bounded in memory; filesystem/WAL/global quotas and crash-orphan lifecycle still require host integration.'])
Path('.codex-work/sealed-export/benchmark.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(summary),flush=True)
