/** Direct ABI validation, sanitizer execution and optional-capability isolation. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import factory from '../../.codex-work/clips/component/mo-skia.mjs';
import oldFactory from '../../.codex-work/image-domain/component/mo-skia.mjs';
const root='.codex-work/clips';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const report=JSON.parse(fs.readFileSync(root+'/parity.json')),images=read(report.images);
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
const words=r=>{const b=read(r);return new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength));};
let triples=0;
function compare(frame,data,status,expected){
 const w=data?raster.rasterImages(frame,data):raster.raster(frame);assert.equal(w.status,status);assert(!raster.invalid);
 for(const asan of [false,true]){
  const h=Buffer.alloc(4),b=Buffer.from(frame.buffer,frame.byteOffset,frame.byteLength),parts=[h,b];h.writeUInt32LE(frame.length);
  if(data){const size=Buffer.alloc(4);size.writeUInt32LE(data.length);parts.push(size,data);}
  const p=spawnSync(root+'/component/mo-skia-probe'+(asan?'-asan':''),data?['--images']:[],{input:Buffer.concat(parts),env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:60000,maxBuffer:90*1024*1024});
  assert.equal(p.status,0,p.stderr.toString());assert.equal(p.stderr.length,0,p.stderr.toString());assert.equal(p.stdout.readUInt32LE(),status);assert.equal(p.stdout.readUInt32LE(4),p.stdout.length-8);
  assert.deepEqual(p.stdout.subarray(8),Buffer.from(w.pixels));if(expected)assert.deepEqual(p.stdout.subarray(8),expected);if(status)assert.equal(p.stdout.length,8);
 }
 triples++;
}
for(const c of report.cases)for(const key of ['frame','sceneFrame'])compare(words(c[key]),c.images?images:null,0,read(c.pixels));
const base=words(report.cases.find(c=>c.name==='solid-branches-zero').frame);
function offsets(f){let p=13;for(let i=0;i<f[5];i++)p+=2+f[p+1]*7;p+=4*f[8];for(let i=0;i<f[9];i++)p+=9+5*f[p+4];p+=f[10]*4+f[11]*14;return {clips:p,draws:p+f[12]*4};}
const o=offsets(base),f=v=>new Uint32Array(new Float32Array([v]).buffer)[0],negatives=[];
for(const [name,index,value,status] of [
 ['clip-count-limit',12,8193,3],['clip-count-length',12,2,1],['parent-self',o.clips,1,1],['parent-forward',o.clips+4,3,1],
 ['clip-path',o.clips+1,base[5],1],['origin-nan',o.clips+2,0x7fc00000,1],['origin-inf',o.clips+3,0x7f800000,1],
 ['origin-range',o.clips+2,f(32769),1],['translated-bounds',o.clips+2,f(32768),3],['draw-clip',o.draws+6,4,1],
 ['image-on-plain',10,1,1],['old-version-new-layout',1,4,1],['bad-fill-rule',13,2,1],
]){const q=base.slice();q[index]=value;compare(q,null,status);negatives.push({name,index,value,status});}
for(const length of [0,1,9,10,12,base.length-1]){compare(base.subarray(0,length),null,1);negatives.push({name:'truncated-'+length,status:1});}
// Empty paths still consume nesting and transition work; no Skia call is needed
// to reject work amplification. Keep input within the outer frame-size budget.
function emptyFrame(nodes,draws,alternate=false){
 const frame=[0x4d4f534b,7,4,4,0,1,draws,0,0,0,0,0,nodes,0,0];
 for(let i=0;i<nodes;i++)frame.push(i,0,0,0);
 for(let i=0;i<draws;i++)frame.push(0,0,0,0xff0000ff,0,0,alternate&&i%2===0?0:nodes);
 return new Uint32Array(frame);
}
compare(emptyFrame(64,65536),null,0);compare(emptyFrame(65,1),null,3);compare(emptyFrame(64,8194,true),null,3);
negatives.push({name:'depth-65',status:3},{name:'applied-empty-clip-work',status:3});
const commands=262144,expensive=[0x4d4f534b,7,4,4,0,1,0,commands,0,0,0,0,5,0,commands];
for(let i=0;i<commands;i++)expensive.push(1,0,0,0,0,0,0);
for(let i=0;i<5;i++)expensive.push(0,0,0,0);
compare(new Uint32Array(expensive),null,3);negatives.push({name:'clip-placement-work',status:3});
const old=await RasterComponent.create(oldFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/image-domain/component/mo-skia.wasm')));
assert(!old.supportsClips);assert.throws(()=>old.raster(base),/clip extension unavailable/);assert(!old.invalid);
const imageFrame=words(report.cases.find(c=>c.images).frame);
assert.throws(()=>old.rasterImages(imageFrame,images),/clip extension unavailable/);assert(!old.invalid);
// V7 can carry no clip nodes and must preserve legacy pixels, including V5's
// whole-image fast path. Proves optional layout upgrade doesn't alter paint.
const oldDomain=JSON.parse(fs.readFileSync('.codex-work/image-domain/parity.json')).cases.find(c=>c.success);
const legacy6=words(oldDomain.frame),oldImages=read(oldDomain.images);
let pos=12;for(let i=0;i<legacy6[5];i++)pos+=2+7*legacy6[pos+1];pos+=4*legacy6[8];for(let i=0;i<legacy6[9];i++)pos+=9+5*legacy6[pos+4];const brush=pos+legacy6[10]*4;pos+=legacy6[10]*4+legacy6[11]*14;
const upgraded=[...legacy6.slice(0,12),0,...legacy6.slice(12,pos)];upgraded[1]=7;
for(let i=pos;i<legacy6.length;i+=6)upgraded.push(...legacy6.slice(i,i+6),0);
compare(new Uint32Array(upgraded),oldImages,0,read(oldDomain.pixels));
const whole=legacy6.slice();whole[brush+10]=f(0);whole[brush+11]=f(0);whole[brush+12]=f(5);whole[brush+13]=f(4);
const legacy5=new Uint32Array([...whole.slice(0,brush+10),...whole.slice(brush+14)]);legacy5[1]=5;
const expected=old.rasterImages(legacy5,oldImages);assert.equal(expected.status,0);
const whole7=new Uint32Array(upgraded);for(let i=0;i<4;i++)whole7[brush+1+10+i]=whole[brush+10+i];compare(whole7,oldImages,0,Buffer.from(expected.pixels));
const result={format:'musteroffice.shared-clips-components/1',triples,successfulNewFrames:report.cases.length*2,negatives,addressSanitizer:true,undefinedBehaviorSanitizer:true,leakSanitizer:false,noDiagnostics:true,oldCapabilityRejections:2,legacyImagePaintComparisons:2,native:entry(root+'/component/mo-skia-probe'),asan:entry(root+'/component/mo-skia-probe-asan'),wasm:entry(root+'/component/mo-skia.wasm'),parity:entry(root+'/parity.json')};
fs.writeFileSync(root+'/components.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({triples,negatives:negatives.length,noDiagnostics:true}));
