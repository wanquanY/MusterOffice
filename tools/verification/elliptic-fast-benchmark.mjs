/** Paired, warm raster-call observations using frozen old and current components. */
import fs from 'node:fs';
import os from 'node:os';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ellipticFrame,floatWord as f} from './elliptic_frames.mjs';
const root='.codex-work/elliptic-fast',out=root+'/benchmark';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const variants=[];
for(const [name,dir] of [['before','.codex-work/elliptic-render/component'],['after',root+'/component']]){
 const build=JSON.parse(fs.readFileSync(dir+'/native-build.json'));
 const libs=['/libmo_skia_adapter.a','/libskia.a','/libpng16.a','/libjpeg.a','/libz.a'].map(s=>{
  const r=build.artifacts.find(r=>r.path.endsWith(s));assert.deepEqual(entry(r.path),r);return r.path;
 });
 const args=['-std=c++20','-O3','-Icomponents/skia','tools/verification/image-domain-bench.cpp',...libs,'-o',out+'/'+name+'-native'];
 const c=spawnSync('clang++',args,{encoding:'utf8'});assert.equal(c.status,0,c.stderr);
 const factory=(await import(pathToFileURL(path.resolve(dir+'/mo-skia.mjs')))).default;
 const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(dir+'/mo-skia.wasm')));
 variants.push({name,raster,native:out+'/'+name+'-native',compile:args,
  artifacts:[entry(dir+'/native-build.json'),entry(dir+'/mo-skia.wasm'),entry(out+'/'+name+'-native')]});
}
const cases=[['native-radial',1024,1024,[0,0,0,0]],['center-point',1024,1024,[0,0,0,0]],
 ['center-circle',1024,1024,[0,0,.3,.3]],['offset-point',512,256,[.6,-.2,0,0]],
 ['center-ellipse',512,256,[0,0,.3,.8]],['nested-offset-ellipse',512,256,[.2,-.15,.25,.6]],
 ['non-nested',512,256,[-.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122]]];
const summary=s=>({samplesMs:s,medianMs:s.toSorted((a,b)=>a-b)[12],p95Ms:s.toSorted((a,b)=>a-b)[23]});
const results=[];
for(const [name,width,height,field] of cases){
 let frame=ellipticFrame({width,height,matrix:[width,0,0,0,height,0],scale:[1,1],field});
 if(name==='native-radial'){
  const a=Array.from(frame);frame=new Uint32Array([...a.slice(0,51),1,0,0,0,3,f(512),f(512),f(512),0,...a.slice(70)]);
 }
 const samples={};let reference;
 for(const v of variants){
  const first=v.raster.raster(frame);assert.equal(first.status,0,name);
  const pixels=Buffer.from(first.pixels);if(reference)assert.deepEqual(pixels,reference,name+' old/new pixels');else reference=pixels;
  const times=[];
  for(let i=0;i<28;++i){
   const start=process.hrtime.bigint(),r=v.raster.raster(frame),ms=Number(process.hrtime.bigint()-start)/1e6;
   assert.equal(r.status,0);assert.deepEqual(Buffer.from(r.pixels),pixels);if(i>=3)times.push(ms);
  }
  const prefix=Buffer.alloc(4),suffix=Buffer.alloc(4);prefix.writeUInt32LE(frame.length);
  const n=spawnSync(v.native,[],{input:Buffer.concat([prefix,Buffer.from(frame.buffer),suffix]),env:{},timeout:120000,maxBuffer:32*1024*1024});
  assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);
  const newline=n.stdout.indexOf(10),data=JSON.parse(n.stdout.subarray(0,newline));assert.deepEqual(n.stdout.subarray(newline+1),pixels);
  samples[v.name]={native:summary(data.samplesMs),wasmAdapter:summary(times)};
 }
 const p=out+'/'+name+'.frame';fs.writeFileSync(p,Buffer.from(frame.buffer));
 const result={name,width,height,field:field.map(Math.fround),frame:entry(p),pixelSha256:sha(reference),...samples};
 result.speedup={native:result.before.native.medianMs/result.after.native.medianMs,wasmAdapter:result.before.wasmAdapter.medianMs/result.after.wasmAdapter.medianMs};
 results.push(result);console.log(JSON.stringify({name,before:result.before.native.medianMs,after:result.after.native.medianMs,speedup:result.speedup}));
}
fs.writeFileSync(root+'/benchmark.json',JSON.stringify({format:'musteroffice.elliptic-fast-benchmark/1',
 environment:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,memoryBytes:os.totalmem(),node:process.version},
 policy:{warmups:3,samples:25,cache:'Warm component; fresh paint/paths/output per call. No GC forcing.',fonts:'none',images:'none',
  order:'Cases sequential; before then after, WASM then Native per variant. Other system load uncontrolled.',
  scope:'Complete low-level raster calls. Native includes validation/allocation/paint/path/pixels; WASM includes TS copies. No Rust/PPTX, startup, RSS or installer.'},
 variants:variants.map(({name,compile,artifacts})=>({name,compile,artifacts})),typescript:entry('.codex-work/elliptic-source/ts-raster/index.js'),results},null,2)+'\n');
