import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import releaseFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import faultFactory from '../../.codex-work/harfbuzz/mo-hb.mjs';
const root='.codex-work/font-metrics',sha=b=>createHash('sha256').update(b).digest('hex');
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const fontPath='fixtures/fonts/owned-metrics-typo.ttf',font=readFileSync(fontPath);
const faultBytes=readFileSync('.codex-work/harfbuzz/mo-hb.wasm'),compiled=new WebAssembly.Module(faultBytes);
const releaseBytes=readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm');
const fresh=()=>faultFactory({wasmBinary:faultBytes});
const regular=await releaseFactory({wasmBinary:releaseBytes});assert.equal(typeof regular._mo_hb_fail_after,'undefined');
const tag=s=>Buffer.from(s).readUInt32BE();
const float=v=>{const b=Buffer.alloc(4);b.writeFloatLE(v);return b.readUInt32LE();};
const metricTags='hasc hdsc hlgp hcla hcld vasc vdsc vlgp hcrs hcrn hcof vcrs vcrn vcof xhgt cpht sbxs sbys sbxo sbyo spxs spys spxo spyo strs stro unds undo'.split(' ').map(tag);
const good=[0x4d4f4d54,1,0,2,28,0,tag('wght'),float(650),tag('wdth'),float(112.5),...metricTags];
function raw(m,words,shaping=false) {
 const allocations=[];let output=0;
 const alloc=n=>{const p=m._malloc(Math.max(n,1));assert.ok(p);allocations.push(p);return p;};
 try {
  const f=alloc(font.length),r=alloc(words.length*4),slots=alloc(8),l=alloc(3);
  m.HEAPU8.set(font,f);m.HEAPU32.set(words,r/4);m.HEAPU8.set(Buffer.from('und'),l);m.HEAPU32.fill(0,slots/4,slots/4+2);
  const status=shaping?m._mo_hb_shape(f,font.length,r,words.length,l,3,slots,slots+4):m._mo_hb_measure_font(f,font.length,r,words.length,slots,slots+4);
  output=m.HEAPU32[slots/4];const count=m.HEAPU32[slots/4+1];assert.equal(Boolean(output),count!==0);
  const result={status,words:output?Array.from(m.HEAPU32.subarray(output/4,output/4+count)):[]};
  if(status!==0)assert.equal(output,0);return result;
 } finally {if(output)m._mo_hb_free(output);for(const p of allocations)m._free(p);}
}
function native(words,name,failAfter,asan=false) {
 const path=root+'/'+name+'.words.bin',bytes=Buffer.alloc(words.length*4);words.forEach((w,i)=>bytes.writeUInt32LE(w>>>0,i*4));writeFileSync(path,bytes);
 const bin='.codex-work/harfbuzz/'+(asan?'asan/':'')+'mo-hb-probe',args=[fontPath,path,'--metrics'];if(failAfter!==undefined)args.push(String(failAfter));
 const r=spawnSync(bin,args,{encoding:'utf8',timeout:30000,env:{...process.env,ASAN_OPTIONS:'halt_on_error=1',UBSAN_OPTIONS:'halt_on_error=1:print_stacktrace=1'}});
 assert.equal(r.status,0,r.stderr);assert.equal(r.stderr,'');return JSON.parse(r.stdout);
}
const inputs=[['valid',good,0],['truncated',good.slice(0,5),1],['trailing',[...good,0],1]];
for(const [name,index,value] of [
 ['magic',0,0],['version',1,2],['bad-face',2,9],['axis-count',3,65],['metric-count',4,29],['reserved',5,1],
 ['unknown-axis',6,tag('xxxx')],['nan-axis',7,0x7fc00000],['infinite-axis',7,0x7f800000],['axis-too-large',7,float(901)],
 ['duplicate-axis',8,tag('wght')],['unknown-metric',10,tag('Oasc')],['duplicate-metric',11,metricTags[0]],
]) {const words=[...good];words[index]=value;inputs.push([name,words,name==='bad-face'?5:1]);}
const cases=[];
for(const [name,words,status] of inputs) {
 const n=native(words,name),s=native(words,name,undefined,true),w=raw(regular,words);
 assert.equal(n.status,status,name);assert.deepEqual(w,{status,words:n.words});assert.deepEqual(s.words,n.words);assert.equal(s.status,status);
 cases.push({name,status,requestSha256:sha(readFileSync(root+'/'+name+'.words.bin')),resultSha256:sha(JSON.stringify(w))});
}
const baseline=raw(regular,good),faults=[];assert.equal(baseline.status,0);
// Minimal actual shaping request for A, sharing this exact variable font.
const shape=[0x4d4f4842,0,4,tag('Latn'),3,0,1,0,1,0,0,16,1,65];
assert.equal(raw(regular,shape,true).status,0);
for(let i=0;i<300;i++) {
 const n=native(good,'fault',i),s=native(good,'fault',i,true),m=await fresh();m._mo_hb_fail_after(i);
 const w=raw(m,good);m._mo_hb_fail_after(0xffffffff);const reuse=raw(m,good),cross=raw(m,shape,true);
 for(const r of [n,s]) {
  assert.ok([0,2].includes(r.status));
  if(r.status===2){assert.deepEqual(r.words,[]);assert.equal(r.recoveryStatus,6);assert.deepEqual(r.recoveryWords,[]);}
  else {assert.deepEqual(r.words,baseline.words);assert.deepEqual(r.recoveryWords,baseline.words);}
 }
 assert.equal(n.status,s.status);assert.ok([0,2].includes(w.status));
 if(w.status===2) {assert.deepEqual(w.words,[]);assert.equal(reuse.status,6);assert.equal(cross.status,6);}
 else {assert.deepEqual(w,baseline);assert.deepEqual(reuse,baseline);assert.equal(cross.status,0);}
 faults.push({failAfter:i,nativeStatus:n.status,wasmStatus:w.status,sanitizedStatus:s.status,wasmShapeReuseStatus:cross.status});
 if(i%10===0)global.gc?.();
}
assert.ok(faults.some(f=>f.nativeStatus===2));assert.ok(faults.some(f=>f.wasmStatus===2));
const shapeFailed=await fresh();shapeFailed._mo_hb_fail_after(0);assert.equal(raw(shapeFailed,shape,true).status,2);shapeFailed._mo_hb_fail_after(0xffffffff);assert.equal(raw(shapeFailed,good).status,6);
// Failure after one successful measurement must remove all partially collected
// metrics, invalidate the adapter and make the other operation unusable too.
let module,calls=0;
const component=await ShapingComponent.create(async options=>{module=await faultFactory(options);const original=module._mo_hb_measure_font;
 module._mo_hb_measure_font=(...args)=>{if(++calls===2)module._mo_hb_fail_after(0);return original(...args);};return module;},compiled);
const q=JSON.parse(readFileSync(root+'/mvar-typo.request.json'));q.instances=q.instances.slice(0,2);const json=JSON.stringify(q);
const failure=JSON.parse(wasm.measure_font(json,font,component));assert.equal(failure.error.code,'COMPONENT_FAILURE');assert.equal(failure.metrics,undefined);assert.equal(component.invalid,true);assert.equal(calls,2);
const reuse=JSON.parse(wasm.measure_font(json,font,component));assert.equal(reuse.error.code,'HOST_FAILURE');
assert.throws(()=>component.shapeBatch(font,Uint32Array.from([0x4d4f5342,1,0x0e0500,0])));
const replacement=await ShapingComponent.create(releaseFactory,new WebAssembly.Module(releaseBytes));assert.equal(JSON.parse(wasm.measure_font(json,font,replacement)).status,'measured');
const report={format:'musteroffice.font-metrics-safety/1',nativeProbeSha256:sha(readFileSync('.codex-work/harfbuzz/mo-hb-probe')),sanitizedProbeSha256:sha(readFileSync('.codex-work/harfbuzz/asan/mo-hb-probe')),
 faultWasmSha256:sha(faultBytes),releaseWasmSha256:sha(releaseBytes),fontSha256:sha(font),addressSanitizer:true,undefinedBehaviorSanitizer:true,cases,faults,
 bidirectionalOperationQuarantine:true,lateBatchFailure:{failure,reuse,partialResult:false,replacementVerified:true},nativeFailures:faults.filter(f=>f.nativeStatus===2).length,wasmFailures:faults.filter(f=>f.wasmStatus===2).length};
writeFileSync(root+'/safety.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({cases:cases.length,faultPositions:faults.length,nativeFailures:report.nativeFailures,wasmFailures:report.wasmFailures}));
