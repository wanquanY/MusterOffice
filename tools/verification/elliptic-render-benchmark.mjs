/** Warm complete raster-call observations. No document/layout/product claim. */
import fs from 'node:fs';import os from 'node:os';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-render/ts-raster/index.js';
import factory from '../../.codex-work/elliptic-render/component/mo-skia.mjs';
import {ellipticFrame,floatWord as f} from './elliptic_frames.mjs';
const root='.codex-work/elliptic-render',out=root+'/benchmark';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const build=JSON.parse(fs.readFileSync(root+'/component/native-build.json'));
const libs=['/libmo_skia_adapter.a','/libskia.a','/libpng16.a','/libjpeg.a','/libz.a'].map(s=>{
 const r=build.artifacts.find(r=>r.path.endsWith(s));assert.deepEqual(entry(r.path),r);return r.path;
});
const args=['-std=c++20','-O3','-Icomponents/skia','tools/verification/image-domain-bench.cpp',...libs,'-o',out+'/native-bench'];
const compiled=spawnSync('clang++',args,{encoding:'utf8'});assert.equal(compiled.status,0,compiled.stderr);
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
const cases=[['native-radial',1024,1024,[0,0,0,0]],['center-point',1024,1024,[0,0,0,0]],
 ['center-circle',1024,1024,[0,0,.3,.3]],['offset-point',512,256,[.6,-.2,0,0]],
 ['center-ellipse',512,256,[0,0,.3,.8]],
 ['non-nested',512,256,[-.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122]]];
const results=[];let reference;
for(const [name,width,height,field] of cases){
 let frame=ellipticFrame({width,height,matrix:[width,0,0,0,height,0],scale:[1,1],field});
 if(name==='native-radial'){
  const before=Array.from(frame);frame=new Uint32Array([...before.slice(0,51),1,0,0,0,3,f(512),f(512),f(512),0,...before.slice(70)]);
 }
 const sample=raster.raster(frame);assert.equal(sample.status,0,name);const pixels=Buffer.from(sample.pixels);
 if(name==='native-radial')reference=pixels;
 if(name==='center-point'){let delta=0;for(let i=0;i<pixels.length;++i)delta=Math.max(delta,Math.abs(pixels[i]-reference[i]));assert(delta<=1);}
 const times=[];
 for(let i=0;i<28;++i){const start=process.hrtime.bigint(),r=raster.raster(frame),ms=Number(process.hrtime.bigint()-start)/1e6;
  assert.equal(r.status,0);assert.deepEqual(Buffer.from(r.pixels),pixels);if(i>=3)times.push(ms);}
 const prefix=Buffer.alloc(4),suffix=Buffer.alloc(4);prefix.writeUInt32LE(frame.length);
 const native=spawnSync(out+'/native-bench',[],{input:Buffer.concat([prefix,Buffer.from(frame.buffer),suffix]),env:{},timeout:120000,maxBuffer:32*1024*1024});
 assert.equal(native.status,0,native.stderr.toString());assert.equal(native.stderr.length,0);
 const newline=native.stdout.indexOf(10),n=JSON.parse(native.stdout.subarray(0,newline));assert.deepEqual(native.stdout.subarray(newline+1),pixels);
 const summary=s=>({samplesMs:s,medianMs:s.toSorted((a,b)=>a-b)[12],p95Ms:s.toSorted((a,b)=>a-b)[23]});
 const path=out+'/'+name+'.frame';fs.writeFileSync(path,Buffer.from(frame.buffer));
 results.push({name,width,height,field:field.map(Math.fround),frame:entry(path),pixelSha256:sha(pixels),native:summary(n.samplesMs),wasmAdapter:summary(times)});
 console.log(JSON.stringify({name,nativeMs:results.at(-1).native.medianMs,wasmMs:results.at(-1).wasmAdapter.medianMs}));
}
const report={format:'musteroffice.elliptic-render-benchmark/1',
 environment:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,memoryBytes:os.totalmem(),node:process.version},
 policy:{warmups:3,samples:25,cache:'Warm component, fresh paint/paths/output per call; no GC forcing.',fonts:'none',images:'none',
 scope:'Actual low-level raster calls. Native C ABI includes validation/allocation/paint/path/pixels, WASM also includes TS allocation/copies. No Rust, PPTX layout, fonts, startup, RSS, installer or product measurement. Runs sequentially; other system load uncontrolled.'},
 nativeCompile:args,artifacts:[entry(root+'/component/native-build.json'),entry(root+'/component/mo-skia.wasm'),entry(root+'/ts-raster/index.js'),entry(out+'/native-bench')],results};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');
