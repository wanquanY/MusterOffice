// Warm, synthetic raster-only observations; no PPTX/product performance claim.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {RasterComponent} from '../../.codex-work/image-domain/ts-raster/index.js';
import factory from '../../.codex-work/image-domain/component/mo-skia.mjs';
const root='.codex-work/image-domain',out=root+'/benchmark';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
// Link against the exact already verified native component closure.
const build=JSON.parse(fs.readFileSync(root+'/component/native-build.json'));
const libs=['/libmo_skia_adapter.a','/libskia.a','/libpng16.a','/libjpeg.a','/libz.a'].map(s=>{
 const r=build.artifacts.find(r=>r.path.endsWith(s));assert.deepEqual(entry(r.path),r);return r.path;
});
const args=['-std=c++20','-O3','-Icomponents/skia','tools/verification/image-domain-bench.cpp',...libs,'-o',out+'/native-bench'];
const compiled=spawnSync('clang++',args,{encoding:'utf8'});assert.equal(compiled.status,0,compiled.stderr);
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
const wasm=createRequire(import.meta.url)('../../.codex-work/image-domain/wasm-node/mo_wasm.js');
let frame;
const backend={rasterImages(f,b){frame=f.slice();return raster.rasterImages(f,b);},invalidate(){raster.invalidate();}};
const full=Buffer.alloc(256*256*4),cropped=Buffer.alloc(192*192*4);
for(let y=0;y<256;y++)for(let x=0;x<256;x++){
 const p=(y*256+x)*4;full.set([x,y,(x+y)%256,255],p);
 if(x>=32&&x<224&&y>=32&&y<224)cropped.set(full.subarray(p,p+4),((y-32)*192+x-32)*4);
}
const q32=n=>String(BigInt(n)<<32n),point=(x,y)=>({x:q32(x),y:q32(y)});
const results=[];
for(const tile of ['clamp','repeat'])for(const sampling of ['nearest','linear']){
 const states=[];
 for(const domain of [false,true]){
  const q=JSON.parse(fs.readFileSync(root+'/cases/domain-0-clamp-clamp-nearest.json'));
  Object.assign(q.raster.viewport,{width:1024,height:768});
  q.raster.paths[0].commands=[{kind:'move',to:point(0,0)},...[[1024,0],[1024,768],[0,768]].map(([x,y])=>({kind:'line',to:point(x,y)})),{kind:'close'}];
  const b=q.raster.draws[0].brush.image;Object.assign(b,{origin:domain?point(-100,-80):point(-4,16),xStep:point(3,0),yStep:point(0,3),tileX:tile,tileY:tile,sampling});
  if(domain)b.sourceDomain={left:q32(32),top:q32(32),right:q32(224),bottom:q32(224)};else delete b.sourceDomain;
  const data=domain?full:cropped,size=domain?256:192;
  q.images=[{width:size,height:size,alpha:'premultiplied',sha256:sha(data)}];
  const r=wasm.render_image_paths(JSON.stringify(q),data,backend);assert.equal(JSON.parse(r.metadata).status,'rendered');
  const pixels=Buffer.from(r.take_pixels()),savedFrame=frame.slice(),samples=[];
  for(let i=0;i<28;i++){
   const start=process.hrtime.bigint(),r=raster.rasterImages(savedFrame,data),elapsed=Number(process.hrtime.bigint()-start)/1e6;
   assert.equal(r.status,0);assert.deepEqual(Buffer.from(r.pixels),pixels);if(i>=3)samples.push(elapsed);
  }
  const words=Buffer.from(savedFrame.buffer),head=Buffer.alloc(4),tail=Buffer.alloc(4);head.writeUInt32LE(savedFrame.length);tail.writeUInt32LE(data.length);
  const native=spawnSync(out+'/native-bench',[],{input:Buffer.concat([head,words,tail,data]),env:{},timeout:60000,maxBuffer:8*1024*1024});
  assert.equal(native.status,0,native.stderr.toString());assert.equal(native.stderr.length,0);
  const end=native.stdout.indexOf(10);assert(end>0);
  const n=JSON.parse(native.stdout.subarray(0,end));assert.equal(n.pixelBytes,pixels.length);
  assert.deepEqual(native.stdout.subarray(end+1),pixels);
  const summary=s=>{const sorted=s.toSorted((a,b)=>a-b);return {samplesMs:s,medianMs:sorted[12],p95Ms:sorted[23]};};
  const name=`${tile}-${sampling}-${domain?'domain':'compact'}`,request=out+'/'+name+'.json';fs.writeFileSync(request,JSON.stringify(q));
  states.push({name,domain,request:entry(request),sourceBytes:data.length,frameWords:savedFrame.length,pixelHash:sha(pixels),
   native:summary(n.samplesMs),wasmAdapter:summary(samples),pixels});
 }
 let delta=0;for(let i=0;i<states[0].pixels.length;i++)delta=Math.max(delta,Math.abs(states[0].pixels[i]-states[1].pixels[i]));
 assert(delta<=1,`${tile}/${sampling}: ${delta}`);
 states.forEach(s=>delete s.pixels);results.push({tile,sampling,maxChannelDeviation:delta,states});
}
const report={format:'musteroffice.image-domain-benchmark/1',
 environment:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,memoryBytes:os.totalmem(),node:process.version},
 policy:{warmups:3,samples:25,viewport:[1024,768],fonts:'none',cache:'warm component; every call validates the resource and allocates/renders output; no GC forcing',
 scope:'Synthetic opaque 256x256 source with integer 192x192 subdomain versus pre-copied compact resource. Copy-to-create-compact cost is excluded; source byte validation differs. Native times C ABI only, WASM includes TS allocation/copies. Excludes Rust/JSON/hashes, startup, fonts, PPTX layout, RSS and product/installer gates. Run order is compact then domain; not statistically controlled.'},
 inputs:{fullSha256:sha(full),croppedSha256:sha(cropped)},nativeCompile:args,
 artifacts:[entry(root+'/component/native-build.json'),entry(root+'/component/mo-skia.wasm'),entry(root+'/ts-raster/index.js'),entry(out+'/native-bench')],results};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(results.map(r=>({tile:r.tile,sampling:r.sampling,delta:r.maxChannelDeviation,states:r.states.map(s=>({domain:s.domain,nativeMs:s.native.medianMs,wasmMs:s.wasmAdapter.medianMs}))}))));
