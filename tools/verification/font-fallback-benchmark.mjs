// Identical mixed-font work, varying only component upload batching.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {performance} from 'node:perf_hooks';
import os from 'node:os';
import {createHarness} from './paragraph-harness.mjs';

assert.equal(typeof global.gc,'function','Run Node with --expose-gc');
const {fixture,wasm,component,sha}=await createHarness('.codex-work/font-fallback');
const f=fixture('A👩'.repeat(32),['notosanssc','notoemoji']);
const json=JSON.stringify(f.request);
let stats;
const invoke=(font,frame)=>{
  stats.calls++;stats.fontBytesUploaded+=font.length;
  return component.shapeBatch(font,frame);
};
const batched={shapeBatch:invoke};
const individual={shapeBatch(font,frame) {
  let offset=4,words=2;
  const replies=[];
  for(let run=0;run<frame[3];run++) {
    const size=2+frame[offset]+frame[offset+1];
    const single=new Uint32Array(4+size);
    single.set(frame.subarray(0,3));single[3]=1;
    single.set(frame.subarray(offset,offset+size),4);offset+=size;
    const reply=invoke(font,single);
    if(reply[0]!==0)return Uint32Array.of(reply[0],run+reply[1]);
    assert.equal(reply[1],1);
    replies.push(reply.subarray(2));words+=reply.length-2;
  }
  assert.equal(offset,frame.length);
  const result=new Uint32Array(words);result.set([0,frame[3]]);offset=2;
  for(const reply of replies){result.set(reply,offset);offset+=reply.length;}
  return result;
}};
const adapters={individual,batched},counts={};
function run(mode,timed=false) {
  stats={calls:0,fontBytesUploaded:0};
  global.gc(); // Deliberately outside the timed interval.
  const start=performance.now(),result=wasm.shape_paragraph(json,f.bundle,adapters[mode]);
  const elapsed=performance.now()-start;
  if(counts[mode])assert.deepEqual(stats,counts[mode]);else counts[mode]={...stats};
  return {result,...(timed?{elapsed}:{})};
}
let expected;
for(let warmup=0;warmup<2;warmup++) {
  const a=run('individual'),b=run('batched');assert.equal(a.result,b.result);
  if(expected)assert.equal(expected,b.result);else expected=b.result;
}
const parsed=JSON.parse(expected);assert.equal(parsed.status,'evaluated');
const fallback=parsed.result.fallback;
assert.equal(fallback.items.length,1);assert.equal(fallback.items[0].fragments.length,64);
assert.ok(fallback.items[0].fragments.every(f=>f.status==='selected'));
assert.equal(fallback.shapingRuns,66);assert.equal(counts.individual.calls,66);assert.equal(counts.batched.calls,4);
const measurements={individual:[],batched:[]};
for(let pair=0;pair<9;pair++)for(const mode of pair%2?['batched','individual']:['individual','batched']) {
  const {result,elapsed}=run(mode,true);assert.equal(result,expected);measurements[mode].push(elapsed);
}
component.invalidate();
const summary=samples=>{const sorted=[...samples].sort((a,b)=>a-b);return {medianMs:sorted[4],minimumMs:sorted[0],maximumMs:sorted.at(-1),samplesMs:samples};};
const result={
  format:'musteroffice.font-fallback-benchmark/1',
  environment:{platform:os.platform(),release:os.release(),architecture:os.arch(),cpu:os.cpus()[0].model,memoryBytes:os.totalmem(),node:process.version},
  scope:'Warm in-memory Node/WASM paragraph preparation with identical input, algorithm and byte-identical results; only the C++ component upload batching differs. Excludes file I/O, component creation, cold start, width fitting, geometry, painting, worker scheduling and the desktop host. Not a full-product performance claim.',
  input:{text:f.request.text,scalarCount:[...f.request.text].length,requestSha256:sha(json),bundleSha256:sha(f.bundle),bundleBytes:f.bundle.length,fonts:f.request.fonts},
  resultSha256:sha(expected),exactResponseVerified:true,shapingRuns:fallback.shapingRuns,fragments:64,
  work:{individual:'Transport proxy splits every component batch into single-run frames; Rust font verification, itemization, probes and boundary reshapes are unchanged.',batched:'Production adapter uploads each candidate font once per batch; both modes still construct C++ face/font for every run.',counts},
  warmupPairs:2,counterbalancedPairs:9,garbageCollection:'Explicit GC before each sample, outside the timed interval. No font-result or cross-request verification cache.',
  individual:summary(measurements.individual),batched:summary(measurements.batched),
  artifacts:{rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cppWasmSha256:sha(readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),adapterSha256:sha(readFileSync('.codex-work/text-component/index.js'))}
};
writeFileSync('.codex-work/font-fallback/benchmark.json',JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({individualMedianMs:result.individual.medianMs,batchedMedianMs:result.batched.medianMs,counts,scope:result.scope}));
