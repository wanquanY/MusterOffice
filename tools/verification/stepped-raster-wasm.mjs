/** Real component + TS owner; all statuses/pixels checked against saved bytes. */
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

const [output,componentPath,tsPath,manifestPath]=process.argv.slice(2);
assert(output&&componentPath&&tsPath&&manifestPath);
await fs.mkdir(output,{recursive:false});
const sha=b=>createHash('sha256').update(b).digest('hex');
const inputs={};
async function load(record) {
  const bytes=await fs.readFile(record.path);
  assert.equal(bytes.length,record.byteLength);assert.equal(sha(bytes),record.sha256);
  inputs[record.path]=record.sha256;return bytes;
}
const {RasterComponent}=await import(pathToFileURL(path.resolve(tsPath,'index.js')));
const {default:factory}=await import(pathToFileURL(path.resolve(componentPath,'mo-skia.mjs')));
const moduleBytes=await fs.readFile(path.join(componentPath,'mo-skia.wasm'));
const module=await WebAssembly.compile(moduleBytes);
const allocations=new Set(),handles=new Set();let started=0,dropped=0;
const component=await RasterComponent.create(async options=>{
  const m=await factory(options);
  const malloc=m._malloc,free=m._free,begin=m._mo_skia_raster_begin,drop=m._mo_skia_raster_drop;
  m._malloc=bytes=>{const p=malloc(bytes);if(p){assert(!allocations.has(p));allocations.add(p);}return p;};
  m._free=p=>{assert(allocations.delete(p),'Release each TS-owned input exactly once');free(p);};
  m._mo_skia_raster_begin=(...args)=>{
    const status=begin(...args),p=m.HEAPU32[args.at(-1)/4];
    if(status===0){assert(p&&!handles.has(p));handles.add(p);started++;}else assert.equal(p,0);
    return status;
  };
  m._mo_skia_raster_drop=p=>{assert(handles.delete(p),'Release each C++ task exactly once');dropped++;drop(p);};
  return m;
},module);
assert(component.supportsExecution);
const old=JSON.parse(await fs.readFile(manifestPath));
const cases=[];
for(const c of old.cases) {
  let raw=await load(c.frame??c.base),frame=new Uint32Array(Uint8Array.from(raw).buffer);
  if(c.mutation) {
    if(c.mutation.name.startsWith('truncated-'))frame=frame.slice(0,Number(c.mutation.name.split('-')[1]));
    else if(c.mutation.index!==undefined)frame[c.mutation.index]=c.mutation.value;
  }
  const expected=await load(c.currentPixels),images=c.images?Uint8Array.from(await load(c.images)):undefined;
  const synchronous=images?component.rasterImages(frame,images):component.raster(frame);
  assert.equal(synchronous.status,c.status,c.name);assert.deepEqual(Buffer.from(synchronous.pixels),expected,c.name);
  for(const units of [1,7,4096]) {
    const hostFrame=frame.slice(),hostImages=images?.slice();
    const begin=component.beginRaster(hostFrame,hostImages);
    let reply=begin,steps=0;
    if(begin.status===0) {
      const execution=begin.execution;
      assert.throws(()=>execution.take(),/incomplete/);
      assert.throws(()=>execution.step(0),/work units/);
      assert.throws(()=>component.beginRaster(frame,images),/unavailable/);
      assert.throws(()=>component.raster(frame),/unavailable/);
      const buffers=[hostFrame.buffer,...(hostImages?[hostImages.buffer]:[])];
      structuredClone(buffers,{transfer:buffers});
      assert.equal(hostFrame.byteLength,0);if(hostImages)assert.equal(hostImages.byteLength,0);
      try {
        while(true) {
          const result=execution.step(units);steps++;
          if(result.status!==0){reply=result;break;}
          if(result.complete) {
            assert(execution.complete);assert.equal(execution.step().complete,true);
            reply=execution.take();assert(execution.closed);
            assert.throws(()=>execution.take(),/closed/);break;
          }
          assert(!execution.complete);assert.throws(()=>execution.take(),/incomplete/);
        }
      } finally {execution.close();execution.close();}
    }
    assert.equal(reply.status,c.status,c.name);assert.deepEqual(Buffer.from(reply.pixels),expected,c.name);
    assert.equal(component.invalid,false);assert.equal(allocations.size,0);assert.equal(handles.size,0);
    cases.push({name:c.name,units,status:reply.status,steps,pixelSha256:sha(reply.pixels)});
  }
}

// Cancel at preparation and drawing boundaries, then actually reuse the healthy
// component. Input and C++ task loans must be balanced on every early close.
const first=old.cases.find(c=>c.name==='source/new/group-inherited-control')??
  old.cases.find(c=>c.name==='prefix/1024x1024');
assert(first,'A real composite frame is required for cancellation checks');
const frameBytes=await load(first.frame),expected=await load(first.currentPixels);
const frame=new Uint32Array(Uint8Array.from(frameBytes).buffer),cancellations=[];
for(const after of [0,1,4,8,12,16]) {
  const start=component.beginRaster(frame);assert.equal(start.status,0);
  let steps=0;
  for(;steps<after;steps++) {const r=start.execution.step();assert.equal(r.status,0);if(r.complete)break;}
  assert(!start.execution.closed);start.execution.close();start.execution.close();assert(start.execution.closed);
  assert.throws(()=>start.execution.step(),/closed/);
  assert.equal(allocations.size,0);assert.equal(handles.size,0);
  const next=component.raster(frame);assert.equal(next.status,0);assert.deepEqual(Buffer.from(next.pixels),expected);
  cancellations.push({after,actualSteps:steps,inputsAndHandlesReleased:true,reusedHealthyComponent:true});
}
for(const [p,h] of Object.entries(inputs))assert.equal(sha(await fs.readFile(p)),h);
assert.equal(started,dropped);
const report={format:'musteroffice.stepped-raster-wasm/1',status:'passed',cases,inputs,cancellations,
  taskHandlesStarted:started,taskHandlesDropped:dropped,inputLoansRemaining:allocations.size,
  wasmSha256:sha(moduleBytes),scope:'Component state transitions with unchanged complete Skia draws; not bounded primitive latency, full playback continuation or product acceptance.'};
await fs.writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({cases:cases.length,taskHandlesStarted:started,taskHandlesDropped:dropped,status:report.status}));
