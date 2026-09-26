/** Frozen raw elliptic, negative and older frame corpus under all new backends. */
import fs from 'node:fs';
import {pixelDelta} from './gradient_coordinate_delta.mjs';
import {ellipticFrame} from './elliptic_frames.mjs';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import factory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
const root='.codex-work/gradient-coordinates',componentRoot=root+'/component';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const words=r=>{const b=load(r);return new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.length));};
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(componentRoot+'/mo-skia.wasm')));
fs.mkdirSync(root+'/components',{recursive:true});
const old=JSON.parse(fs.readFileSync('.codex-work/elliptic-render/components.json')),records=[];
function compare(name,frame,status,pixels,data=null,inputs={}){
 const w=data?component.rasterImages(frame,data):component.raster(frame);assert.equal(w.status,status,name);assert(!component.invalid);
 const current=Buffer.from(w.pixels),delta=pixelDelta(current,pixels);
 if(!name.startsWith('elliptic/')&&!name.startsWith('rect-gradient/')&&!name.startsWith('gradient-field/')&&!name.startsWith('source/'))assert.deepEqual(current,pixels,name);
 const output=root+'/components/'+records.length+'.rgba';fs.writeFileSync(output,current);
 for(const suffix of ['', '-asan']){
  const h=Buffer.alloc(4),parts=[h,Buffer.from(frame.buffer,frame.byteOffset,frame.byteLength)];h.writeUInt32LE(frame.length);
  if(data){const b=Buffer.alloc(4);b.writeUInt32LE(data.length);parts.push(b,data);}
  const n=spawnSync(componentRoot+'/mo-skia-probe'+suffix,data?['--images']:[],
   {input:Buffer.concat(parts),env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:120000,maxBuffer:1<<27});
  assert.equal(n.status,0,name+': '+n.stderr.toString());assert.equal(n.stderr.length,0);
  assert.equal(n.stdout.readUInt32LE(),status);assert.equal(n.stdout.length,8+n.stdout.readUInt32LE(4));
  assert.deepEqual(n.stdout.subarray(8),current,name);
 }
 records.push({name,status,...inputs,currentPixels:entry(output),delta});
}
for(const c of old.cases)compare('elliptic/'+c.name,words(c.frame),c.expectedStatus,c.pixels?load(c.pixels):Buffer.alloc(0),null,{frame:c.frame,...(c.pixels?{pixels:c.pixels}:{})});
const base=words(old.cases[0].frame);
for(const c of old.negatives){
 const frame=c.name.startsWith('truncated-')?base.slice(0,Number(c.name.slice(10))):base.slice();
 if(c.index!==undefined)frame[c.index]=c.value;
 compare('invalid/'+c.name,frame,c.status,Buffer.alloc(0),null,{mutation:c,base:old.cases[0].frame});
}
compare('recovery',base,0,load(old.cases[0].pixels),null,{frame:old.cases[0].frame,pixels:old.cases[0].pixels});
let legacyFrames=0,sourceFrames=0;
for(const name of ['rect-gradient','office-gradient','gradient-field','compositing','clips']){
 const legacy=JSON.parse(fs.readFileSync('.codex-work/'+name+'/parity.json'));
 for(const c of legacy.cases)for(const key of ['frame','sceneFrame'])if(c[key]){
  compare(name+'/'+c.name+'/'+key,words(c[key]),0,load(c.pixels),c.images?load(legacy.images):null,
   {frame:c[key],pixels:c.pixels,...(c.images?{images:legacy.images}:{})});legacyFrames++;
 }
}
const source=JSON.parse(fs.readFileSync('.codex-work/rect-gradient/source-parity.json'));
for(const c of source.cases.filter(c=>c.name.startsWith('new/'))){
 compare('source/'+c.name,words(c.frame),0,load(c.pixels),Buffer.alloc(0),{frame:c.frame,pixels:c.pixels});sourceFrames++;
}
assert.equal(records.length,old.triples);
const legacyTriples=records.length;
for(const [name,width,height,status] of [['exact-below-budget',256,256,0],['exact-over-budget',257,257,3]]){
 const frame=ellipticFrame({width,height,matrix:[1,0,2**-70,0,1,2**-70],tile:[2,2],scale:[1,1],field:[0,0,2,2]});
 const pixels=status?Buffer.alloc(0):Buffer.from(component.raster(frame).pixels);
 const framePath=root+'/components/'+name+'.frame';fs.writeFileSync(framePath,Buffer.from(frame.buffer));
 compare(name,frame,status,pixels,null,{frame:entry(framePath)});
}
compare('after-budget-recovery',base,0,load(old.cases[0].pixels),null,{frame:old.cases[0].frame,pixels:old.cases[0].pixels});
fs.writeFileSync(root+'/components.json',JSON.stringify({format:'musteroffice.gradient-coordinate-components/1',triples:records.length,legacyTriples,legacyFrames,sourceFrames,
 cases:records,prior:entry('.codex-work/elliptic-render/components.json'),failureRecovery:true,
 addressSanitizer:true,undefinedBehaviorSanitizer:true,leakSanitizer:false,
 native:entry(componentRoot+'/mo-skia-probe'),asan:entry(componentRoot+'/mo-skia-probe-asan'),wasm:entry(componentRoot+'/mo-skia.wasm')},null,2)+'\n');
console.log(JSON.stringify({triples:records.length,legacyFrames,sourceFrames}));
