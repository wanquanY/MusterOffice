/** Re-execute actual Rust-generated V12 batches under Native/WASM and sanitizers. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import factory from '../../.codex-work/elliptic-render/component/mo-skia.mjs';
const root='.codex-work/elliptic-source',componentRoot='.codex-work/elliptic-render/component';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(componentRoot+'/mo-skia.wasm')));
const records=[];
for(const [name,images] of [['runtime',false],['source-parity',true]]){
 const report=JSON.parse(fs.readFileSync(root+'/'+name+'.json'));
 for(const c of report.cases){
  if(!c.frame||c.name.startsWith('prior'))continue;
  const b=load(c.frame),f=new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.length));assert.equal(f[1],12);
  const w=images?component.rasterImages(f,new Uint8Array()):component.raster(f),status=c.status==='rendered'?0:3;
  assert.equal(w.status,status);assert(!component.invalid);
  if(c.pixels)assert.deepEqual(Buffer.from(w.pixels),load(c.pixels));else assert.equal(w.pixels.length,0);
  for(const suffix of ['', '-asan']){
   const h=Buffer.alloc(4);h.writeUInt32LE(f.length);const data=images?Buffer.concat([h,b,Buffer.alloc(4)]):Buffer.concat([h,b]);
   const n=spawnSync(componentRoot+'/mo-skia-probe'+suffix,images?['--images']:[],{input:data,env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:60000,maxBuffer:1<<27});
   assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);
   assert.equal(n.stdout.readUInt32LE(),status);assert.equal(n.stdout.length,n.stdout.readUInt32LE(4)+8);
   assert.deepEqual(n.stdout.subarray(8),Buffer.from(w.pixels));
  }
  records.push({name:c.name,images,status,frame:c.frame,...(c.pixels?{pixels:c.pixels}:{})});
 }
}
assert.equal(records.length,83);
fs.writeFileSync(root+'/components.json',JSON.stringify({format:'musteroffice.elliptic-source-components/1',triples:records.length,cases:records,addressSanitizer:true,undefinedBehaviorSanitizer:true,leakSanitizer:false,native:entry(componentRoot+'/mo-skia-probe'),asan:entry(componentRoot+'/mo-skia-probe-asan'),wasm:entry(componentRoot+'/mo-skia.wasm')},null,2)+'\n');console.log(JSON.stringify({triples:records.length}));
