/** Actual V12 pixels, failures, ownership, older capabilities and byte regressions. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-render/ts-raster/index.js';
import factory from '../../.codex-work/elliptic-render/component/mo-skia.mjs';
import oldFactory from '../../.codex-work/rect-gradient/component/mo-skia.mjs';
const root='.codex-work/elliptic-render';fs.mkdirSync(root+'/cases',{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const words=r=>{const b=load(r);return new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength));};
import {ellipticFrame as frame,floatWord as f} from './elliptic_frames.mjs';
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
assert(component.supportsEllipticGradients);
let triples=0;
function compare(frame,status=0,expected=null,data=null){
 const w=data?component.rasterImages(frame,data):component.raster(frame);assert.equal(w.status,status);assert(!component.invalid);
 for(const asan of [false,true]){
  const h=Buffer.alloc(4),parts=[h,Buffer.from(frame.buffer,frame.byteOffset,frame.byteLength)];h.writeUInt32LE(frame.length);
  if(data){const b=Buffer.alloc(4);b.writeUInt32LE(data.length);parts.push(b,data);}
  const n=spawnSync(root+'/component/mo-skia-probe'+(asan?'-asan':''),data?['--images']:[],
    {input:Buffer.concat(parts),env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:120000,maxBuffer:1<<27});
  assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);
  assert.equal(n.stdout.readUInt32LE(),status);assert.equal(n.stdout.length,8+n.stdout.readUInt32LE(4));
  assert.deepEqual(n.stdout.subarray(8),Buffer.from(w.pixels));
  if(expected)assert.deepEqual(n.stdout.subarray(8),expected);if(status)assert.equal(n.stdout.length,8);
 }
 triples++;return Buffer.from(w.pixels);
}

const cases=[];
function test(name,options={},expectedStatus=0){
 const q=frame(options);const pixels=compare(q,expectedStatus);
 const path=root+'/cases/'+name;fs.writeFileSync(path+'.frame',Buffer.from(q.buffer));
 const record={name,options,expectedStatus,frame:entry(path+'.frame')};
 if(!expectedStatus){fs.writeFileSync(path+'.rgba',pixels);record.pixels=entry(path+'.rgba');}
 cases.push(record);console.log(JSON.stringify({name,status:expectedStatus}));return q;
}
const base=test('center-point');
test('concentric-circle',{field:[0,0,.3,.3]});
test('near-whole-circle',{field:[0,0,Math.fround(1-2**-24),Math.fround(1-2**-24)]});
test('offset-point',{field:[.6,-.2,0,0]});test('external-point',{field:[2,0,0,0]});
test('center-ellipse',{field:[0,0,.3,.8]});test('offset-ellipse',{field:[-.2,.15,.25,.6]});
test('external-ellipse',{field:[1.3,.2,.4,.2]});
test('non-nested',{field:[-.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122]});
test('first-of-three',{width:32,height:16,matrix:[1,0,32767,0,1,32767],
 scale:[.9187984697040693,.32614082270717],
 field:[-.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122]});
test('vertical-line',{field:[0,0,0,.6]});test('horizontal-line',{field:[0,0,.6,0]});
test('whole-circle',{field:[0,0,1,1]});test('expanded',{field:[0,0,2,1.5]});
test('mirror',{matrix:[32,0,-16,0,16,-8],tile:[2,2],field:[.2,-.1,.3,.5]});
test('repeat',{matrix:[32,0,-16,0,16,-8],tile:[1,1],field:[.2,-.1,.3,.5]});
test('shear',{matrix:[128,32,-32,0,64,0],field:[.2,-.1,.3,.5]});
test('flipped',{matrix:[-128,0,128,0,64,0],field:[.2,-.1,.3,.5]});
test('tail',{width:131,height:67});test('office-ramp',{office:true});
test('precision-failure',{width:1,height:1,matrix:[1,0,0,0,.5,0],scale:[1,2**-149],field:[32768,0,32768,1]},3);
test('sample-budget',{width:64,height:64,field:[0,0,2,2],draws:4097},3);
test('node-budget',{width:64,height:64,matrix:[1,0,32767,0,1,32767],
 scale:[.9187984697040693,.32614082270717],
 field:[-.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122],draws:410},3);
compare(base,0,load(cases[0].pixels)); // Numerical failures preserve the instance.
const g=51,negatives=[];
for(const [name,index,value,status] of [
 ['old-frame',1,11,1],['unknown-kind',g,5,1],['negative-scale',g+13,f(-1),1],
 ['zero-scale',g+14,0,1],['large-scale',g+13,f(1.01),1],['negative-radius',g+17,f(-1),1],
 ['large-center',g+15,f(65536),1],['nan-radius',g+18,0x7fc00000,1],['infinite-center',g+16,f(Infinity),1],
 ['bad-tile',g+11,3,1],['singular-plane',g+5,0,3],['bad-color',g+20,0x7fc00000,1],
]){const q=base.slice();q[index]=value;compare(q,status);negatives.push({name,index,value,status});}
for(const n of [0,13,g+9,g+17,g+18,g+19,base.length-1]){compare(base.slice(0,n),1);negatives.push({name:'truncated-'+n,status:1});}
const old=await RasterComponent.create(oldFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/rect-gradient/component/mo-skia.wasm')));
assert(!old.supportsEllipticGradients);assert.throws(()=>old.raster(base),/elliptic gradient extension unavailable/);
assert.throws(()=>old.rasterImages(base,new Uint8Array()),/elliptic gradient extension unavailable/);assert(!old.invalid);
let legacyFrames=0;
for(const path of ['.codex-work/rect-gradient/parity.json','.codex-work/office-gradient/parity.json',
 '.codex-work/gradient-field/parity.json','.codex-work/compositing/parity.json','.codex-work/clips/parity.json']){
 const legacy=JSON.parse(fs.readFileSync(path));
 for(const c of legacy.cases)for(const key of ['frame','sceneFrame'])if(c[key]){
  compare(words(c[key]),0,load(c.pixels),c.images?load(legacy.images):null);legacyFrames++;
 }
}
const source=JSON.parse(fs.readFileSync('.codex-work/rect-gradient/source-parity.json'));
let sourceFrames=0;
for(const c of source.cases.filter(c=>c.name.startsWith('new/'))){compare(words(c.frame),0,load(c.pixels),Buffer.alloc(0));sourceFrames++;}
const report={format:'musteroffice.elliptic-render-components/1',triples,cases,negatives,legacyFrames,sourceFrames,
 oldCapabilityRejections:2,failureRecovery:true,addressSanitizer:true,undefinedBehaviorSanitizer:true,leakSanitizer:false,
 native:entry(root+'/component/mo-skia-probe'),asan:entry(root+'/component/mo-skia-probe-asan'),wasm:entry(root+'/component/mo-skia.wasm')};
fs.writeFileSync(root+'/components.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({triples,legacyFrames,sourceFrames,negatives:negatives.length}));
