/** Direct V8 grammar, native/WASM/ASan parity and real bounded-heap failure. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/compositing/ts-raster/index.js';
import factory from '../../.codex-work/compositing/component/mo-skia.mjs';
import oldFactory from '../../.codex-work/clips/component/mo-skia.mjs';
const root='.codex-work/compositing';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const words=r=>{const b=load(r);return new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength));};
const compiled=new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm'));
const raster=await RasterComponent.create(factory,compiled),report=JSON.parse(fs.readFileSync(root+'/parity.json')),images=load(report.images);
let triples=0;
function compare(frame,data,status,expected){
 const w=data?raster.rasterImages(frame,data):raster.raster(frame);assert.equal(w.status,status);assert(!raster.invalid);
 for(const asan of [false,true]){
  const h=Buffer.alloc(4),parts=[h,Buffer.from(frame.buffer,frame.byteOffset,frame.byteLength)];h.writeUInt32LE(frame.length);
  if(data){const s=Buffer.alloc(4);s.writeUInt32LE(data.length);parts.push(s,data);}
  const n=spawnSync(root+'/component/mo-skia-probe'+(asan?'-asan':''),data?['--images']:[],{input:Buffer.concat(parts),env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:60000,maxBuffer:90*1024*1024});
  assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);assert.equal(n.stdout.readUInt32LE(),status);assert.equal(n.stdout.readUInt32LE(4),n.stdout.length-8);
  assert.deepEqual(n.stdout.subarray(8),Buffer.from(w.pixels));if(expected)assert.deepEqual(n.stdout.subarray(8),expected);if(status)assert.equal(n.stdout.length,8);
 }
 triples++;
}
for(const c of report.cases)for(const k of ['frame','sceneFrame'])compare(words(c[k]),c.images?images:null,0,load(c.pixels));
function offsets(f){let p=14;for(let i=0;i<f[5];i++)p+=2+7*f[p+1];p+=4*f[8];for(let i=0;i<f[9];i++)p+=9+5*f[p+4];p+=4*f[10]+14*f[11]+4*f[12];return {snapshots:p,draws:p+f[13]};}
const base=words(report.cases[0].frame),o=offsets(base),negatives=[];
for(const [name,index,value,status] of [
 ['snapshot-limit',13,65,3],['snapshot-count-length',13,2,1],['capture-after-end',o.snapshots,base[6],1],
 ['reference-before-capture',o.draws+5,1,1],['blend-invalid',o.draws+7,2,1],['snapshot-reference',o.draws+2*8+5,2,1],
 ['plain-images',10,1,1],['clip-limit',12,8193,3],['old-version-new-layout',1,7,1],
]){const q=base.slice();q[index]=value;if(name==='reference-before-capture')q[o.draws+3]=0;compare(q,null,status);negatives.push({name,index,value,status});}
const many=words(report.cases.find(c=>c.name==='three-prefixes-immutable').frame),mo=offsets(many);
for(const [name,index,value] of [['duplicate-prefix',mo.snapshots+1,0],['unsorted-prefix',mo.snapshots,2]]){
 const q=many.slice();q[index]=value;compare(q,null,1);negatives.push({name,index,value,status:1});
}
const over=many.slice();over[2]=8192;over[3]=2048;compare(over,null,3);negatives.push({name:'snapshot-byte-budget',status:3});
for(const n of [0,1,9,10,12,13,base.length-1]){compare(base.subarray(0,n),null,1);negatives.push({name:'truncated-'+n,status:1});}
// Capture zero is a real viewport clear, including when geometry is empty.
const empty=n=>new Uint32Array([0x4d4f534b,8,4,4,0,1,n,0,0,0,0,0,0,n,0,0,...Array.from({length:n},(_,i)=>i),...Array.from({length:n},(_,i)=>[0,0,0,0,0,i+1,0,1]).flat()]);
compare(empty(64),null,0);compare(empty(65),null,3);negatives.push({name:'65-distinct-prefixes',status:3});
// Source-only composition has no pixel capture allocation.
const sourceOnly=base.slice();sourceOnly[13]=0;sourceOnly[o.draws+2*8+3]=0x800000ff;sourceOnly[o.draws+2*8+5]=0;
const sourceFrame=new Uint32Array([...sourceOnly.slice(0,o.snapshots),...sourceOnly.slice(o.snapshots+1)]);
compare(sourceFrame,null,0,load(report.cases[0].pixels));
const old=await RasterComponent.create(oldFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/clips/component/mo-skia.wasm')));
assert(!old.supportsCompositing);assert.throws(()=>old.raster(base),/compositing extension unavailable/);assert(!old.invalid);
const imageCase=report.cases.find(c=>c.images);assert.throws(()=>old.rasterImages(words(imageCase.frame),images),/compositing extension unavailable/);assert(!old.invalid);
// Exercise the exact old V7 frames, not a regenerated approximation.
const legacy=JSON.parse(fs.readFileSync('.codex-work/clips/parity.json'));let legacyFrames=0;
for(const c of legacy.cases)for(const k of ['frame','sceneFrame']){compare(words(c[k]),c.images?load(legacy.images):null,0,load(c.pixels));legacyFrames++;}
// Real Emscripten heap exhaustion. First prove a 64 MiB output fits with the
// same retained 128 MiB host reservation and no capture; then add its snapshot.
// No allocator hooks or production fault-injection code are used.
async function pressure(capture){
 let module;const c=await RasterComponent.create(async opts=>(module=await factory(opts)),compiled);
 assert(module._malloc(128*1024*1024));
 const f=new Uint32Array(capture?[0x4d4f534b,8,8192,2048,0,1,1,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,1,0,1]:[0x4d4f534b,8,8192,2048,0,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);
 if(capture){
  let failed=false;
  try{const r=c.raster(f);failed=r.status===2;assert.equal(r.pixels.length,0);}catch(e){assert.match(String(e),/trap|unreachable|abort|allocation/i);failed=true;}
  assert(failed&&c.invalid);assert.throws(()=>c.raster(base),/unavailable/);
  return {capture:true,invalidated:true,partialPixelsPublished:false};
 }
 const r=c.raster(f);assert.equal(r.status,0);assert.equal(r.pixels.length,67108864);c.invalidate();return {capture:false,outputBytes:r.pixels.length};
}
const heapControl=await pressure(false),heapFailure=await pressure(true);
const result={format:'musteroffice.compositing-components/1',triples,legacyFrames,negatives,oldCapabilityRejections:2,heapControl,heapFailure,addressSanitizer:true,undefinedBehaviorSanitizer:true,leakSanitizer:false,noDiagnostics:true,native:entry(root+'/component/mo-skia-probe'),asan:entry(root+'/component/mo-skia-probe-asan'),wasm:entry(root+'/component/mo-skia.wasm'),parity:entry(root+'/parity.json')};
fs.writeFileSync(root+'/components.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({triples,legacyFrames,negatives:negatives.length,heapFailure}));
