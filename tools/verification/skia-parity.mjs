import assert from 'node:assert/strict';
import fs from 'node:fs';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';

const root='.codex-work/skia/verification', fixtures=JSON.parse(fs.readFileSync(root+'/fixtures.json'));
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const wasmBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm'), compiled=new WebAssembly.Module(wasmBytes);
let raw, observedImports;
const component=await RasterComponent.create(async options => {
  raw=await factory({...options,instantiateWasm(imports,receive){
    observedImports=imports;return options.instantiateWasm(imports,receive);
  }});return raw;
},compiled);
const native='.codex-work/skia/mo-skia-probe', asan='.codex-work/skia/mo-skia-probe-asan';
const records=[];
let maxDifference=0, differentBytes=0, oraclePixels=0;
function nativeCall(path,bytes){
  const prefix=Buffer.alloc(4);prefix.writeUInt32LE(bytes.length/4);
  const run=spawnSync(path,[],{input:Buffer.concat([prefix,bytes]),maxBuffer:96*1024*1024,timeout:15000,
    env:{...process.env,ASAN_OPTIONS:'detect_leaks=0:abort_on_error=1',UBSAN_OPTIONS:'halt_on_error=1'}});
  assert.equal(run.status,0,run.stderr?.toString());assert(!run.error);assert.equal(run.stderr.length,0,run.stderr.toString());
  assert(run.stdout.length>=8);const status=run.stdout.readUInt32LE(0),length=run.stdout.readUInt32LE(4);
  assert.equal(run.stdout.length,8+length);return {status,pixels:run.stdout.subarray(8)};
}
function oracle(name,x,y){
  let inside=false;
  if(name==='empty')return [0,0,0,0];
  if(name==='background')return [17,34,51,255];
  if(name==='transparent-background')return [9,17,26,128];
  if(name==='solid')inside=x>=8&&x<38&&y>=12&&y<32;
  if(name==='translated-clip')inside=x<32&&y<28;
  if(['winding-solid','winding-hole','evenodd-hole'].includes(name)){
    inside=x>=8&&x<56&&y>=8&&y<56;
    if(name!=='winding-solid'&&x>=20&&x<44&&y>=20&&y<44)inside=false;
  }
  if(name==='half-pixel'){
    const coverage=(y>=8&&y<28)?Math.max(0,Math.min(x+1,38.5)-Math.max(x,8.5)):0;
    const alpha=Math.round(coverage*255);return [alpha,0,0,alpha];
  }
  if(name==='alpha-overlap'){
    const red=x>=8&&x<40&&y>=8&&y<40,blue=x>=24&&x<56&&y>=24&&y<56;
    return red&&blue?[64,0,128,192]:red?[128,0,0,128]:blue?[0,0,128,128]:[0,0,0,0];
  }
  return inside?[255,0,0,255]:[0,0,0,0];
}
for(const c of fixtures.cases){
  const input=fs.readFileSync(c.requestPath);assert.equal(sha(input),c.requestSha256);
  const frame=Uint32Array.from({length:input.length/4},(_,i)=>input.readUInt32LE(i*4));
  const n=nativeCall(native,input),s=nativeCall(asan,input),w=component.raster(frame);
  assert.equal(n.status,c.status,c.name);assert.equal(w.status,c.status,c.name);
  assert.equal(s.status,n.status,c.name);assert.deepEqual(s.pixels,n.pixels,c.name+' sanitizer output');
  let difference=0,changed=0;
  if(w.status===0){
    assert.equal(w.pixels.length,n.pixels.length);assert.equal(w.width,frame[2]);assert.equal(w.height,frame[3]);
    for(let i=0;i<n.pixels.length;i++){
      const d=Math.abs(n.pixels[i]-w.pixels[i]);difference=Math.max(difference,d);if(d)changed++;
      if(i%4<3){assert(n.pixels[i]<=n.pixels[i-i%4+3]);assert(w.pixels[i]<=w.pixels[i-i%4+3]);}
    }
    // This bounded component corpus gate is not the presentation visual profile.
    assert(difference<=2,`${c.name}: pixel-channel difference ${difference}`);
    if(c.oracle){
      for(let y=0;y<w.height;y++)for(let x=0;x<w.width;x++){
        const expected=oracle(c.oracle,x,y);oraclePixels++;
        for(let k=0;k<4;k++)assert(Math.abs(n.pixels[(y*w.width+x)*4+k]-expected[k])<=1,`${c.name} oracle ${x},${y},${k}`);
      }
    }
    fs.writeFileSync(root+'/'+c.name+'.rgba',n.pixels);
    fs.writeFileSync(root+'/'+c.name+'.wasm.rgba',w.pixels);
  }else assert.equal(n.pixels.length,0);
  maxDifference=Math.max(maxDifference,difference);differentBytes+=changed;
  records.push({...c,byteLength:n.pixels.length,nativeSha256:sha(n.pixels),wasmSha256:sha(w.status===0?w.pixels:Buffer.alloc(0)),
    maxChannelDifference:difference,differentBytes:changed,sanitizerMatches:true});
}
assert(!component.invalid);
// Pixel output is an independent host copy, not a live view of component memory.
const empty=Uint32Array.of(0x4d4f534b,4,2,2,0xff332211,0,0,0,0,0);
const snapshot=component.raster(empty);assert.equal(snapshot.status,0);const saved=sha(snapshot.pixels);
component.raster(Uint32Array.of(0x4d4f534b,4,2,2,0xffffffff,0,0,0,0,0));assert.equal(sha(snapshot.pixels),saved);
await assert.rejects(()=>RasterComponent.create(async()=>raw,compiled),/already owned/);
const envSlots=raw._malloc(8);assert(envSlots);
raw.HEAPU32.fill(0xffffffff,envSlots/4,envSlots/4+2);
assert.equal(observedImports.wasi_snapshot_preview1.environ_sizes_get(envSlots,envSlots+4),0);
assert.deepEqual([...raw.HEAPU32.subarray(envSlots/4,envSlots/4+2)],[0,0]);raw._free(envSlots);
assert.equal(observedImports.wasi_snapshot_preview1.environ_get(0,0),0);
for(const name of ['fd_close','fd_write','fd_seek'])assert.throws(()=>observedImports.wasi_snapshot_preview1[name](),/denied/);
assert.throws(()=>observedImports.env._tzset_js(),/denied/);

// Deliberately corrupt the result boundary of an otherwise real module.
// These are host protocol tests, separate from the allocator tests below.
const protocol=[];
for(const mode of ['status','failure-with-output','short-image','out-of-range-pointer']){
  let m;
  const guarded=await RasterComponent.create(async options=>{m=await factory(options);return m;},compiled);
  m._mo_skia_raster=(_r,_n,out,len)=>{
    m.HEAPU32[out/4]=mode==='out-of-range-pointer'?m.HEAPU8.length+4:16;
    m.HEAPU32[len/4]=mode==='short-image'?4:16;
    return mode==='status'?9:mode==='failure-with-output'?1:0;
  };
  assert.throws(()=>guarded.raster(empty),/Invalid raster/);assert(guarded.invalid);
  assert.throws(()=>guarded.raster(empty),/unavailable/);protocol.push(mode);
}

// Exhaust the real wasm heap after reserving a request and output slots.
// A large output allocation must fail, produce no image, and invalidate the module.
let exhaustedRaw;
const exhausted=await RasterComponent.create(async options=>{exhaustedRaw=await factory(options);return exhaustedRaw;},compiled);
const held=[];
while(true){const p=exhaustedRaw._malloc(1024*1024);if(!p)break;held.push(p);}
assert(held.length>100);
const oom=exhausted.raster(Uint32Array.of(0x4d4f534b,4,4096,4096,0,0,0,0,0,0));
assert.equal(oom.status,2);assert(exhausted.invalid);assert.throws(()=>exhausted.raster(empty),/unavailable/);
await assert.rejects(()=>RasterComponent.create(async()=>exhaustedRaw,compiled),/already owned/);
const replacement=await RasterComponent.create(factory,compiled);assert.equal(replacement.raster(empty).status,0);

// Reserve transport slots, then exhaust the actual allocator with only 128 bytes
// returned. The output pixel allocation fits, while an upstream Skia allocation
// aborts. Supplying the reserved transport storage ensures this reaches Skia.
let trappedRaw;
const trapped=await RasterComponent.create(async options=>{trappedRaw=await factory(options);return trappedRaw;},compiled);
const reservedRequest=trappedRaw._malloc(40),reservedSlots=trappedRaw._malloc(8),headroom=trappedRaw._malloc(128);
assert(reservedRequest&&reservedSlots&&headroom);
let exhaustionBytes=0;
for(const size of [1048576,65536,1024,64])while(true){
  const p=trappedRaw._malloc(size);if(!p)break;exhaustionBytes+=size;
}
trappedRaw._free(headroom);
const queue=[reservedRequest,reservedSlots];
trappedRaw._malloc=()=>{assert(queue.length);return queue.shift();};
let trapMessage='';
try{trapped.raster(Uint32Array.of(0x4d4f534b,4,1,1,0,0,0,0,0,0));assert.fail('expected real Skia allocation trap');}
catch(error){trapMessage=error.message;assert.match(trapMessage,/trap or ambient access denied: _abort_js/);}
assert.equal(queue.length,0);assert(trapped.invalid);
assert.equal(trappedRaw.HEAPU32[reservedSlots/4],0);assert.equal(trappedRaw.HEAPU32[reservedSlots/4+1],0);
assert.throws(()=>trapped.raster(empty),/unavailable/);

const report={format:'musteroffice.skia-component-parity/1',nativeSha256:sha(fs.readFileSync(native)),
  asanSha256:sha(fs.readFileSync(asan)),wasmSha256:sha(wasmBytes),glueSha256:sha(fs.readFileSync('.codex-work/skia/mo-skia.mjs')),
  adapterSha256:sha(fs.readFileSync('.codex-work/raster-component/index.js')),cases:records,
  oraclePixels,maxChannelDifference:maxDifference,differentBytes,paragraphMapping:fixtures.paragraphMapping,
  imports:WebAssembly.Module.imports(compiled),actualAllocationFailure:{heldMiB:held.length,status:oom.status,
    noPartialOutput:true,reuseRejected:true,sameModuleRewrapRejected:true,replacementVerified:true},
  actualSkiaAllocationTrap:{exhaustionBytes,trapMessage,outputSlotsZero:true,reuseRejected:true},
  hostCopyOwnershipVerified:true,explicitEmptyEnvironmentVerified:true,ambientIoDenied:true,protocolFailures:protocol};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({cases:records.length,oraclePixels,maxDifference,differentBytes,actualAllocationFailure:report.actualAllocationFailure}));
