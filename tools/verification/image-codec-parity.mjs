import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const root=process.argv[2]??'.codex-work/image-codec';
const cases=process.argv[3]??root+'/cases';
const componentRoot=process.argv[4]??'.codex-work/image-codec/component';
const adapter=process.argv[5]??'.codex-work/image-codec/ts-raster/index.js';
const wasmRoot=process.argv[6]??root+'/wasm-node';
const {RasterComponent}=await import(pathToFileURL(path.resolve(adapter)));
const {default:factory}=await import(pathToFileURL(path.resolve(componentRoot+'/mo-skia.mjs')));
const wasm=createRequire(import.meta.url)(path.resolve(wasmRoot+'/mo_wasm.js'));
const previous=process.argv[7]?JSON.parse(fs.readFileSync(process.argv[7])):null;
const module=new WebAssembly.Module(fs.readFileSync(componentRoot+'/mo-skia.wasm'));
const component=await RasterComponent.create(factory,module);
assert(component.supportsDecode && component.supportsImages);
const out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const hash=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:hash(b)};};
let calls=0;
const backend={decodeImage(b){calls++;return component.decodeImage(b);},
 rasterImages(f,b){calls++;return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
function run(request,encoded,scene=false){
 const h=Buffer.alloc(8);h.writeUInt32LE(Buffer.byteLength(request));h.writeUInt32LE(encoded.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',[scene?'--image-scene':'--decode-image'],{input:Buffer.concat([h,Buffer.from(request),encoded]),env:{},maxBuffer:90*1024*1024,timeout:60000});
 assert.equal(n.status,0,n.stderr.toString());
 const length=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+length).toString(),pixels=n.stdout.subarray(8+length);
 assert.equal(pixels.length,n.stdout.readUInt32LE(4));calls=0;
 const w=scene?wasm.render_image_scene(request,encoded,backend):wasm.decode_image(request,encoded,backend);
 assert.equal(w.metadata,metadata);
 assert.deepEqual(Buffer.from(w.take_pixels()),pixels);
 assert(!component.invalid);
 return {metadata,pixels,response:JSON.parse(metadata),calls};
}
const reports=[];
const rendered=[];
for(const c of JSON.parse(fs.readFileSync(cases+'/manifest.json'))){
 const b=fs.readFileSync(c.path);assert.equal(hash(b),c.sha256);
 const r=run(JSON.stringify({sourceSha256:c.sha256}),b);
 assert.equal(r.calls,1);
 if(c.expected){
  assert.equal(r.response.status,'decoded',c.name);
  const info=r.response.info,expected=fs.readFileSync(c.expected);
  assert.equal(info.width,c.width);assert.equal(info.height,c.height);assert.equal(info.orientation,c.orientation);
  assert.equal(info.sourceSha256,c.sha256);assert.equal(info.pixelsSha256,hash(r.pixels));
  if(c.color)assert.equal(info.sourceColor,c.color);
  if(c.resolution)assert.deepEqual(info.resolution,c.resolution,c.name);
  assert.equal(r.pixels.length,expected.length);
  let max=0;
  for(let i=0;i<expected.length;i++)max=Math.max(max,Math.abs(expected[i]-r.pixels[i]));
  assert(max<=c.tolerance,`${c.name}: channel deviation ${max}`);
  fs.writeFileSync(out+'/'+c.name+'.rgba',r.pixels);
  const q=structuredClone(JSON.parse(fs.readFileSync('.codex-work/image-scene/cases/premultiplied-nearest-clamp-nested.json')));
  const point=(x,y)=>({x:String(BigInt(x)<<32n),y:String(BigInt(y)<<32n)});
  q.raster.viewport.width=info.width;q.raster.viewport.height=info.height;
  q.raster.scene.paths[0].commands=[{kind:'move',to:point(0,0)},
    ...[[info.width,0],[info.width,info.height],[0,info.height]].map(([x,y])=>({kind:'line',to:point(x,y)})),{kind:'close'}];
  q.raster.scene.transforms=[{parent:null,affine:{linear:['4294967296','0','0','4294967296'],translation:point(0,0)}}];
  const instance=q.raster.scene.instances[0];instance.transform=0;
  Object.assign(instance.brush.image,{origin:point(0,0),xStep:point(1,0),yStep:point(0,1)});
  q.images=[{width:info.width,height:info.height,alpha:'premultiplied',sha256:info.pixelsSha256}];
  const scene=run(JSON.stringify(q),r.pixels,true);
  assert.equal(scene.calls,1);assert.equal(scene.response.status,'rendered');assert.deepEqual(scene.pixels,r.pixels);
  fs.writeFileSync(out+'/'+c.name+'.scene.json',JSON.stringify(q));
  fs.writeFileSync(out+'/'+c.name+'.scene-response.json',scene.metadata);
  rendered.push({name:c.name,request:entry(out+'/'+c.name+'.scene.json'),response:entry(out+'/'+c.name+'.scene-response.json'),pixels:entry(out+'/'+c.name+'.rgba')});
 }else{ assert.equal(r.response.status,'error',c.name);assert.equal(r.response.error.code,c.code,c.name);assert.equal(r.pixels.length,0); }
 if(previous){
  const old=previous.cases.find(x=>x.name===c.name);assert(old,c.name);
  const before=JSON.parse(fs.readFileSync(old.response.path)),after=structuredClone(r.response);
  // This stage extends decoded metadata. All pre-existing fields, failures,
  // pixel bytes and post-decode scene results must remain unchanged.
  if(after.status==='decoded'&&!Object.hasOwn(before.info,'resolution'))delete after.info.resolution;
  assert.deepEqual(after,before,c.name);
  if(old.pixels)assert.deepEqual(r.pixels,fs.readFileSync(old.pixels.path),c.name);
 }
 fs.writeFileSync(out+'/'+c.name+'.json',r.metadata);
 reports.push({name:c.name,input:entry(c.path),response:entry(out+'/'+c.name+'.json'),
  ...(c.expected?{expected:entry(c.expected),pixels:entry(out+'/'+c.name+'.rgba'),tolerance:c.tolerance}:{} )});
}
const bytes=fs.readFileSync(cases+'/png-plain.bin'),digest=hash(bytes);
for(const request of ['{}',JSON.stringify({sourceSha256:'0'.repeat(64)}),JSON.stringify({sourceSha256:digest,extra:1}),
 `{"sourceSha256":"${digest}","sourceSha256":"${digest}"}`]){
 const r=run(request,bytes);assert.equal(r.response.status,'error');assert.equal(r.calls,0);assert.equal(r.pixels.length,0);
 reports.push({negativeRequest:request,response:r.response});
}
let injected=0;
for(const reply of [{status:0,words:new Uint32Array(0),pixels:new Uint8Array(0)},
 {status:0,words:new Uint32Array([1,1,1,1,1,1,8,0,4]),pixels:new Uint8Array(8)},
 {status:NaN,words:new Uint32Array(9),pixels:new Uint8Array(0)},
 {status:5,words:new Uint32Array([1,0,0,0,0,0,0,0,0]),pixels:new Uint8Array(0)}]){
 let invalid=false;
 const r=wasm.decode_image(JSON.stringify({sourceSha256:digest}),bytes,{decodeImage(){return reply;},invalidate(){invalid=true;}});
 assert.equal(JSON.parse(r.metadata).error.code,'COMPONENT_INVALID');assert.equal(r.take_pixels().length,0);assert(invalid);injected++;
}
const result={format:'musteroffice.image-codec-parity/1',pairedCalls:reports.length+rendered.length,decodeCalls:reports.length,sceneCalls:rendered.length,badHostReplies:injected,
 imports:WebAssembly.Module.imports(module),component:entry(componentRoot+'/mo-skia.wasm'),cases:reports,rendered,
 ...(previous?{previousReport:entry(process.argv[7]),priorFieldsAndPixelsUnchanged:true}:{})};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({pairedCalls:result.pairedCalls,decodeCalls:reports.length,sceneCalls:rendered.length,badHostReplies:injected}));
