import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {RasterComponent} from '../../.codex-work/image-brush/ts-raster/index.js';
import factory from '../../.codex-work/image-brush/component/mo-skia.mjs';
const root='.codex-work/image-brush', sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const module=new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm'));
const raster=await RasterComponent.create(factory,module);
const wasm=createRequire(import.meta.url)('../../.codex-work/image-brush/wasm-node/mo_wasm.js');
assert(raster.supportsImages);
const manifest=JSON.parse(fs.readFileSync(root+'/cases/manifest.json')), records=[];
let runtimeCalls=0;
const backend={rasterImages(frame,images){runtimeCalls++;return raster.rasterImages(frame,images);},
 invalidate(){raster.invalidate();}};
function runtime(json,images){
 const header=Buffer.alloc(8);header.writeUInt32LE(Buffer.byteLength(json));header.writeUInt32LE(images.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',['--images'],{input:Buffer.concat([header,Buffer.from(json),images]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());const size=n.stdout.readUInt32LE(),raw=n.stdout.subarray(8,8+size).toString(),pixels=n.stdout.subarray(8+size);
 assert.equal(n.stdout.readUInt32LE(4),pixels.length);runtimeCalls=0;
 const w=wasm.render_image_paths(json,images,backend);
 assert.equal(w.metadata,raw);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);
 const response=JSON.parse(raw);assert.equal(runtimeCalls,response.status==='rendered'?1:0);
 return {raw,pixels,response};
}
function probe(frame,images,asan=false){
 const raw=Buffer.alloc(8+frame.length*4);raw.writeUInt32LE(frame.length);
 frame.forEach((v,i)=>raw.writeUInt32LE(v,4+i*4));raw.writeUInt32LE(images.length,raw.length-4);
 const p=spawnSync(root+'/component/mo-skia-probe'+(asan?'-asan':''),['--images'],{
  input:Buffer.concat([raw,images]),env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(p.status,0,p.stderr?.toString());assert.equal(p.stderr.length,0,p.stderr.toString());
 assert.equal(p.stdout.readUInt32LE(4),p.stdout.length-8);return {status:p.stdout.readUInt32LE(),pixels:p.stdout.subarray(8)};
}
let base;
for(const c of [...manifest.cases,...manifest.negative]){
 const prefix=root+'/cases/'+c.name,json=fs.readFileSync(prefix+'.json'),images=fs.readFileSync(prefix+'.bin'),head=Buffer.alloc(4);head.writeUInt32LE(json.length);
 const p=spawnSync('target/debug/examples/image_raster',[],{input:Buffer.concat([head,json,images]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(p.status,0,p.stderr?.toString());const size=p.stdout.readUInt32LE(),metadata=p.stdout.subarray(4,4+size),info=JSON.parse(metadata),pixels=p.stdout.subarray(4+size);
 fs.writeFileSync(prefix+'.result.json',metadata);
 const r=runtime(json.toString(),images);assert.deepEqual(r.pixels,pixels,c.name);
 fs.writeFileSync(prefix+'.runtime.json',r.raw);
 const record={name:c.name,request:entry(prefix+'.json'),images:entry(prefix+'.bin'),response:entry(prefix+'.result.json'),runtime:entry(prefix+'.runtime.json')};
 if(info.error){assert(c.name.startsWith('invalid-'));assert.equal(info.calls,0);assert.equal(pixels.length,0);assert.equal(info.invalid,false);}
 else{
  assert(!c.name.startsWith('invalid-'));assert.equal(info.calls,1);assert.equal(info.invalid,false);
  assert.equal(sha(pixels),info.raster.sha256);assert.equal(sha(images),info.resourcesSha256);
  assert.deepEqual(r.response.info,{raster:info.raster,images:info.images,resourcesSha256:info.resourcesSha256});
  const frame=Uint32Array.from(info.frame),w=raster.rasterImages(frame,images);
  assert.equal(w.status,0,c.name);assert.deepEqual(Buffer.from(w.pixels),pixels,c.name);
  for(const asan of [false,true]){const n=probe(frame,images,asan);assert.equal(n.status,0,c.name);assert.deepEqual(n.pixels,pixels,c.name);}
  fs.writeFileSync(prefix+'.rgba',pixels);record.pixels=entry(prefix+'.rgba');
  if(c.name==='premultiplied-nearest-clamp')base={frame,images};
 }
 records.push(record);
}
// Parse only the fixture's known frame to locate its image sections; mutations
// directly target component grammar and do not depend on Rust preflight.
const {frame,images}=base;let offset=12;
for(let i=0;i<frame[5];i++)offset+=2+7*frame[offset+1];
offset+=4*frame[8];for(let i=0;i<frame[9];i++)offset+=9+5*frame[offset+4];
const descriptor=offset,brush=offset+4*frame[10];
const f=v=>{const x=new Float32Array([v]);return new Uint32Array(x.buffer)[0];};
const mutations=[
 ['version',a=>a[1]=4,1],['resource-count',a=>a[10]=4097,3],['brush-count',a=>a[11]=4097,3],
 ['offset',a=>a[descriptor]=4,1],['width-zero',a=>a[descriptor+1]=0,1],['dimensions',a=>a[descriptor+1]=8193,3],
 ['alpha',a=>a[descriptor+3]=2,1],['reference',a=>a[brush]=1,1],['tile-x',a=>a[brush+1]=4,1],
 ['tile-y',a=>a[brush+2]=4,1],['sampling',a=>a[brush+3]=2,1],['nan',a=>a[brush+4]=f(NaN),1],
 ['inf',a=>a[brush+5]=f(Infinity),1],['singular',a=>a[brush+4]=f(0),3],
 ['small-scale',a=>a[brush+4]=f(1/32768),3],['corner-range',a=>a[brush+4]=f(20000),3],
 ['shader-reference',a=>a[a.length-1]=2,1],['shader-color',a=>a[a.length-3]=1,1],
];
const invalid=[];
function invalidCase(name,a,b,status){
 const w=raster.rasterImages(a,b);assert.equal(w.status,status,name);assert.equal(w.pixels.length,0);
 for(const asan of [false,true]){const n=probe(a,b,asan);assert.equal(n.status,status,name);assert.equal(n.pixels.length,0);}
 assert(!raster.invalid);invalid.push({name,status});
}
for(const [name,mutate,status]of mutations){const a=frame.slice();mutate(a);invalidCase(name,a,images,status);}
invalidCase('short-frame',frame.slice(0,-1),images,1);
invalidCase('trailing-frame',Uint32Array.from([...frame,0]),images,1);
invalidCase('short-bundle',frame,images.subarray(0,-1),1);
invalidCase('trailing-bundle',frame,Buffer.concat([images,Buffer.from([0])]),1);
const bad=Buffer.from(images);bad[3]=0;invalidCase('premul-channels',frame,bad,1);
assert.equal(raster.rasterImages(frame,images).status,0);
assert.throws(()=>raster.rasterImages(frame,new Uint8Array(new SharedArrayBuffer(16))),/independent/);
assert(!raster.invalid);
for(const sampling of ['nearest','linear'])for(const tile of ['clamp','repeat','mirror','decal']){
 assert.deepEqual(fs.readFileSync(`${root}/cases/straight-${sampling}-${tile}.rgba`),
  fs.readFileSync(`${root}/cases/premultiplied-${sampling}-${tile}.rgba`));
}
assert.deepEqual(fs.readFileSync(root+'/cases/straight-hidden-color.rgba'),fs.readFileSync(root+'/cases/premultiplied-hidden-color.rgba'));
const good=JSON.parse(fs.readFileSync(root+'/cases/premultiplied-nearest-clamp.json'));
const jsonNegatives=[
 ['unknown',q=>{q.extra=true;}],['unknown-image',q=>{q.images[0].extra=true;}],
 ['unknown-brush',q=>{q.raster.draws[0].brush.image.extra=true;}],
 ['bad-sampling',q=>{q.raster.draws[0].brush.image.sampling='cubic';}],
 ['numeric-coordinate',q=>{q.raster.draws[0].brush.image.origin.x=0;}],
 ['missing-digest',q=>{delete q.images[0].sha256;}],
];
for(const [name,mutate]of jsonNegatives){const q=structuredClone(good);mutate(q);const r=runtime(JSON.stringify(q),images);assert.equal(r.response.status,'error',name);}
assert.equal(runtime('{"images":[],"images":[]}',images).response.status,'error');
let invalidated=0;
const failed=wasm.render_image_paths(JSON.stringify(good),images,{
 rasterImages(){return {status:0,pixels:new Uint8Array([255,0,0,0])};},invalidate(){invalidated++;}});
assert.equal(JSON.parse(failed.metadata).error.code,'COMPONENT_INVALID');assert.equal(failed.take_pixels().length,0);assert.equal(invalidated,1);
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.image-brush-parity/1',
 counts:{rustRequests:records.length,imageCases:manifest.cases.length,preflightFailures:manifest.negative.length,componentRejections:invalid.length,jsonRejections:7,hostFailureRejections:1},
 cases:records,invalid,componentInputs:[entry(root+'/component/mo-skia.wasm'),entry(root+'/component/mo-skia-probe'),entry(root+'/component/mo-skia-probe-asan')]},null,2));
console.log(JSON.stringify({rustRequests:records.length,imageCases:manifest.cases.length,componentRejections:invalid.length}));
