/** Direct grammar, lifetime and sanitizer checks below Rust's preflight. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/image-domain/ts-raster/index.js';
import factory from '../../.codex-work/image-domain/component/mo-skia.mjs';
import oldFactory from '../../.codex-work/image-codec/component/mo-skia.mjs';
const root='.codex-work/image-domain';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
function probe(frame,bytes,asan){
 const words=Buffer.from(frame.buffer,frame.byteOffset,frame.byteLength),head=Buffer.alloc(4),size=Buffer.alloc(4);
 head.writeUInt32LE(frame.length);size.writeUInt32LE(bytes.length);
 const p=spawnSync(root+'/component/mo-skia-probe'+(asan?'-asan':''),['--images'],{
  input:Buffer.concat([head,words,size,bytes]),env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:60000,maxBuffer:90*1024*1024});
 assert.equal(p.status,0,p.stderr?.toString());assert.equal(p.stderr.length,0,p.stderr.toString());
 assert.equal(p.stdout.readUInt32LE(4),p.stdout.length-8);return {status:p.stdout.readUInt32LE(),pixels:p.stdout.subarray(8)};
}
function compare(frame,bytes,status,expected){
 const w=raster.rasterImages(frame,bytes);assert.equal(w.status,status);assert(!raster.invalid);
 for(const asan of [false,true]){
  const n=probe(frame,bytes,asan);assert.equal(n.status,status);assert.deepEqual(n.pixels,Buffer.from(w.pixels));
  if(expected)assert.deepEqual(n.pixels,expected);if(status)assert.equal(n.pixels.length,0);
 }
}
const good=JSON.parse(fs.readFileSync(root+'/parity.json')).cases.filter(c=>c.success);
for(const c of good){const b=read(c.frame);compare(new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength)),read(c.images),0,read(c.pixels));}
const b=read(good[0].frame),base=new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength)),bytes=read(good[0].images);
let p=12;for(let i=0;i<base[5];i++)p+=2+7*base[p+1];p+=4*base[8];
for(let i=0;i<base[9];i++)p+=9+5*base[p+4];const brush=p+4*base[10];
const f=v=>new Uint32Array(new Float32Array([v]).buffer)[0];
const mutations=[
 ['nan-left',brush+10,0x7fc00000,1],['infinite-right',brush+12,0x7f800000,1],
 ['range-left',brush+10,f(-32769),1],['range-bottom',brush+13,f(32769),1],
 ['empty-x',brush+12,base[brush+10],3],['reverse-y',brush+13,f(-10),3],
 ['projected-range',brush+12,f(20000),3],['bad-tile',brush+1,4,1],['bad-filter',brush+3,2,1],
 ['bad-resource',brush,base[10],1],['version',1,7,1],['brush-count',11,2,1],
 ['legacy-wrong-layout',1,5,1],
];
const negative=[];
for(const [name,index,value,status] of mutations){const frame=base.slice();frame[index]=value;compare(frame,bytes,status);negative.push({name,status,index,value});}
compare(base.subarray(0,base.length-1),bytes,1);negative.push({name:'truncated',status:1});
const tiny=base.slice();tiny[brush+10]=f(0);tiny[brush+12]=f(1/32768);compare(tiny,bytes,3);negative.push({name:'tiny-domain',status:3});
const old=await RasterComponent.create(oldFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/image-codec/component/mo-skia.wasm')));
assert(old.supportsImages);assert(!old.supportsImageDomains);assert.throws(()=>old.rasterImages(base,bytes),/source domain extension unavailable/);assert(!old.invalid);
// Explicit full-domain V6 must agree with both current and legacy V5 execution.
const full=base.slice();full[brush+10]=f(0);full[brush+11]=f(0);full[brush+12]=f(5);full[brush+13]=f(4);
const legacy=new Uint32Array([...full.subarray(0,brush+10),...full.subarray(brush+14)]);legacy[1]=5;
const prior=old.rasterImages(legacy,bytes);assert.equal(prior.status,0);
compare(full,bytes,0,Buffer.from(prior.pixels));compare(legacy,bytes,0,Buffer.from(prior.pixels));
const result={format:'musteroffice.image-domain-components/1',successfulFrames:good.length,negative,
 currentNativeWasmSanitizerTriples:good.length+negative.length+2,legacyCapabilityAndWholeImageChecks:3,
 addressSanitizer:true,undefinedBehaviorSanitizer:true,leakSanitizer:false,noDiagnostics:true,
 nativeProbe:entry(root+'/component/mo-skia-probe'),asanProbe:entry(root+'/component/mo-skia-probe-asan'),
 component:entry(root+'/component/mo-skia.wasm'),parity:entry(root+'/parity.json')};
fs.writeFileSync(root+'/components.json',JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({successfulFrames:good.length,negativeFrames:negative.length,triples:result.currentNativeWasmSanitizerTriples,noDiagnostics:true}));
