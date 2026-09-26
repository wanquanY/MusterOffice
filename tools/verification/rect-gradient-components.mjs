/** V11 rectangular grammar and actual Native/WASM/ASan/UBSan, plus V10/V9/V8/V7 frames. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/rect-gradient/ts-raster/index.js';
import factory from '../../.codex-work/rect-gradient/component/mo-skia.mjs';
import oldFactory from '../../.codex-work/office-gradient/component/mo-skia.mjs';
const root='.codex-work/rect-gradient';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const words=r=>{const b=load(r);return new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength));};
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
const report=JSON.parse(fs.readFileSync(root+'/parity.json'));
let triples=0,legacyFrames=0,sourceFrames=0;
function compare(frame,data,status,expected){
 const w=data?component.rasterImages(frame,data):component.raster(frame);assert.equal(w.status,status);assert(!component.invalid);
 for(const asan of [false,true]){
  const h=Buffer.alloc(4),parts=[h,Buffer.from(frame.buffer,frame.byteOffset,frame.byteLength)];h.writeUInt32LE(frame.length);
  if(data){const b=Buffer.alloc(4);b.writeUInt32LE(data.length);parts.push(b,data);}
  const n=spawnSync(root+'/component/mo-skia-probe'+(asan?'-asan':''),data?['--images']:[],{input:Buffer.concat(parts),env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:60000,maxBuffer:1<<26});
  assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);assert.equal(n.stdout.readUInt32LE(),status);assert.equal(n.stdout.length,8+n.stdout.readUInt32LE(4));assert.deepEqual(n.stdout.subarray(8),Buffer.from(w.pixels));if(expected)assert.deepEqual(n.stdout.subarray(8),expected);if(status)assert.equal(n.stdout.length,8);
 }
 triples++;
}
for(const c of report.cases)for(const key of ['frame','sceneFrame'])if(c[key])compare(words(c[key]),null,0,load(c.pixels));
const base=words(report.cases[0].frame);let g=14;for(let i=0;i<base[5];i++)g+=2+base[g+1]*7;g+=base[8]*4;
const f=n=>new Uint32Array(new Float32Array([n]).buffer)[0],negatives=[];
for(const [name,index,value,status] of [
 ['old-frame-rect',1,10,1],['unknown-kind',g,4,1],['interpolation',g+2,3,1],
 ['nan-matrix',g+5,0x7fc00000,1],['singular-plane',g+5,f(0),3],
 ['unknown-axis-tile',g+11,3,1],['negative-rate',g+13,f(-1),1],['nan-rate',g+14,0x7fc00000,1],['infinite-rate',g+15,f(Infinity),1],['range-rate',g+16,f(2**95),1],
 ['stop-count',g+4,4097,3],['color-nan',g+18,0x7fc00000,1],['negative-alpha',g+21,f(-.1),1],
]){const q=base.slice();q[index]=value;compare(q,null,status);negatives.push({name,index,value,status});}
for(const size of [0,1,10,13,g+9,g+16,g+17,g+26,base.length-1]){compare(base.slice(0,size),null,1);negatives.push({name:'truncated-'+size,status:1});}
const office=words(report.cases.find(c=>c.name==='mirror-point-officeGamma1875').frame);
for(const [name,index,value] of [['premul-office',g+3,1],['shift-first-office',g+17,f(.1)],['shift-last-office',g+22,f(.99)]]){
 const q=office.slice();q[index]=value;compare(q,null,1);negatives.push({name,index,value,status:1});
}
const old=await RasterComponent.create(oldFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/office-gradient/component/mo-skia.wasm')));
assert(!old.supportsRectGradients);assert.throws(()=>old.raster(base),/rectangular gradient extension unavailable/);assert(!old.invalid);
assert.throws(()=>old.rasterImages(base,new Uint8Array()),/rectangular gradient extension unavailable/);assert(!old.invalid);
for(const path of ['.codex-work/office-gradient/parity.json','.codex-work/gradient-field/parity.json','.codex-work/compositing/parity.json','.codex-work/clips/parity.json']){
 const legacy=JSON.parse(fs.readFileSync(path));for(const c of legacy.cases)for(const key of ['frame','sceneFrame'])if(c[key]){compare(words(c[key]),c.images?load(legacy.images):null,0,load(c.pixels));legacyFrames++;}
}
const compatible=JSON.parse(fs.readFileSync('.codex-work/office-gradient/parity.json')).cases[0];
assert.deepEqual(Buffer.from(old.raster(words(compatible.frame)).pixels),load(compatible.pixels));assert(!old.invalid);
const source=JSON.parse(fs.readFileSync(root+'/source-parity.json'));
for(const c of source.cases.filter(c=>c.name.startsWith('new/'))){compare(words(c.frame),Buffer.alloc(0),0,load(c.pixels));sourceFrames++;}
assert.equal(sourceFrames,18);
const result={format:'musteroffice.rect-gradient-components/1',triples,legacyFrames,sourceFrames,negatives,oldCapabilityRejections:2,addressSanitizer:true,undefinedBehaviorSanitizer:true,leakSanitizer:false,noDiagnostics:true,native:entry(root+'/component/mo-skia-probe'),asan:entry(root+'/component/mo-skia-probe-asan'),wasm:entry(root+'/component/mo-skia.wasm')};
fs.writeFileSync(root+'/components.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({triples,legacyFrames,sourceFrames,negatives:negatives.length}));
