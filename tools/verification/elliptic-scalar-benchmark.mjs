// Standalone scalar batches. No path rasterization, colors, layout, fonts or I/O
// in timed sections. Actual production solver, same bytes on Native and WASM.
import fs from 'node:fs/promises';
import os from 'node:os';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {performance} from 'node:perf_hooks';
import create from '../../.codex-work/elliptic-scalar/probe.mjs';
const root='.codex-work/elliptic-scalar/';
const width=128,height=128,count=width*height;
const fields=[['center-point',[0,0,0,0]],['offset-point',[.6,-.2,0,0]],
  ['center-ellipse',[0,0,.3,.8]],['external-point',[2,0,0,0]],
  ['non-nested',[-.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122]],
  ['expanded',[0,0,2,1.5]]];
const module=await create();
const results=[];
const summary=times=>({samplesMs:times,minimumMs:Math.min(...times),
  medianMs:[...times].sort((a,b)=>a-b)[3],maximumMs:Math.max(...times)});
for(const [name,field] of fields) {
  const bytes=Buffer.alloc(count*28);
  for(let y=0;y<height;++y) for(let x=0;x<width;++x) {
    const i=y*width+x;
    const values=[2*(x+.5)/width-1,2*(y+.5)/height-1,...field];
    values.forEach((v,k)=>bytes.writeFloatLE(v,i*28+k*4)); bytes.writeUInt32LE(512,i*28+24);
  }
  await fs.writeFile(root+'benchmark-'+name+'.bin',bytes);
  const prefix=Buffer.alloc(4); prefix.writeUInt32LE(count);
  const native=spawnSync(root+'native-probe',['benchmark'],{input:Buffer.concat([prefix,bytes])});
  assert.equal(native.status,0,native.stderr.toString()); assert.equal(native.stderr.length,0);
  const n=JSON.parse(native.stdout);
  const a=module._malloc(bytes.length),b=module._malloc(count*40); assert.ok(a&&b);
  let checksum=2166136261;const times=[],statuses=[0,0,0,0];
  try {
    module.HEAPU32.set(new Uint32Array(bytes.buffer,bytes.byteOffset,bytes.length/4),a/4);
    for(let trial=0;trial<8;++trial) {
      const start=performance.now(); module._mo_elliptic_probe(a,b,count); const ms=performance.now()-start;
      if(trial) times.push(ms);
    }
    const output=module.HEAPU32.subarray(b/4,b/4+count*10);
    for(const word of output) checksum=Math.imul(checksum^word,16777619)>>>0;
    for(let i=0;i<count;++i) { assert.ok(output[i*10]<4); ++statuses[output[i*10]]; }
  } finally { module._free(a);module._free(b); }
  assert.equal(checksum,n.checksum);assert.deepEqual(statuses,n.statuses);
  assert.deepEqual(statuses,[count,0,0,0]);
  results.push({name,field:field.map(Math.fround),count,width,height,statuses,checksum,
    native:summary(n.milliseconds),wasm:summary(times)});
  console.log(JSON.stringify({name,nativeMedianMs:results.at(-1).native.medianMs,wasmMedianMs:results.at(-1).wasm.medianMs}));
}
const output={format:'musteroffice.elliptic-scalar-benchmark/1',
  machine:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,
    logicalCpus:os.cpus().length,memoryBytes:os.totalmem(),node:process.version},
  method:'One untimed warmup and seven timed batches, synchronous single thread; median of all seven. Binary32 unit-square cell-center grid. No font or image inputs. Allocations, file I/O and hashing outside timing. Other system activity not controlled.',
  scope:'Scalar microbenchmark only. Includes probe output packing, excludes actual drawing, source geometry, colors, pixel buffers and application startup. Not an end-to-end latency/throughput or installer measurement.',results};
await fs.writeFile(root+'benchmark.json',JSON.stringify(output,null,2)+'\n');
