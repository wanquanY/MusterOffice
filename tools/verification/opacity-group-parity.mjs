/** Owned compositing examples through the public Rust/TS/native boundaries. */
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {solid,point,U} from './path-raster-fixtures.mjs';
const [output,worker,wasmPath,componentPath,tsPath,probe]=process.argv.slice(2);
assert(output&&worker&&wasmPath&&componentPath&&tsPath&&probe);
await fs.mkdir(output,{recursive:false});
const {RasterComponent}=await import(pathToFileURL(path.resolve(tsPath,'index.js')));
const {default:factory}=await import(pathToFileURL(path.resolve(componentPath,'mo-skia.mjs')));
const wasm=createRequire(import.meta.url)(path.resolve(wasmPath,'mo_wasm.js'));
const wasmBytes=await fs.readFile(path.join(componentPath,'mo-skia.wasm'));
const component=await RasterComponent.create(factory,await WebAssembly.compile(wasmBytes));
assert(component.supportsOpacityGroups);
const sha=b=>createHash('sha256').update(b).digest('hex');
async function save(name,bytes) {
 const file=path.join(output,name);await fs.writeFile(file,bytes,{flag:'wx'});
 return {path:file,byteLength:bytes.length,sha256:sha(bytes)};
}
const native=(request,scene=false)=>{
 const bytes=Buffer.from(JSON.stringify(request)),h=Buffer.alloc(4);h.writeUInt32LE(bytes.length);
 const n=spawnSync(worker,scene?['--scene']:[],{input:Buffer.concat([h,bytes]),env:{},timeout:30000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());const m=n.stdout.readUInt32LE(0),p=n.stdout.readUInt32LE(4);
 assert.equal(n.stdout.length,8+m+p);
 return {metadata:n.stdout.subarray(8,8+m).toString(),pixels:n.stdout.subarray(8+m)};
};
let captured,calls=0;
const backend={raster(frame){calls++;captured=frame.slice();return component.raster(frame);},invalidate(){component.invalidate();}};
const group=(firstDraw,endDraw,opacity=32768)=>({firstDraw,endDraw,opacity});
function overlapping(){
 const q=solid();q.viewport.background=[255,255,255,255];
 q.draws.push({...structuredClone(q.draws[0]),origin:point(12n*U,8n*U),brush:{kind:'solid',rgba:[0,0,255,255]}});
 q.opacityGroups=[group(0,2)];return q;
}
const pixel=(bytes,x,y,w=64)=>Array.from(bytes.subarray((y*w+x)*4,(y*w+x)*4+4));
const merge=(s,d,a)=>{
 const source=Array.from(s,(v)=>Math.round(v*a/65535)),inv=255-source[3];
 return source.map((v,k)=>v+Math.round(d[k]*inv/255));
};
const cases=[],requests=[];
for(const alpha of [0,1,32768,65534,65535]) {
 const q=overlapping();q.opacityGroups[0].opacity=alpha;
 requests.push({name:'overlap-'+alpha,q,oracle:p=>{
  assert.deepEqual(pixel(p,25,25),merge([0,0,255,255],[255,255,255,255],alpha));
  assert.deepEqual(pixel(p,10,10),merge([255,0,0,255],[255,255,255,255],alpha));
  assert.deepEqual(pixel(p,0,0),[255,255,255,255]);
 }});
}
{
 const q=overlapping();q.opacityGroups=[group(0,2,40000),group(1,2,25000)];
 requests.push({name:'nested',q,oracle:p=>assert.deepEqual(pixel(p,25,25),merge(merge([0,0,255,255],[255,0,0,255],25000),[255,255,255,255],40000))});
}
{
 const q=overlapping();q.opacityGroups=[group(0,1,20000),group(1,2,40000)];
 requests.push({name:'siblings',q,oracle:p=>{
  assert.deepEqual(pixel(p,25,25),merge([0,0,255,255],merge([255,0,0,255],[255,255,255,255],20000),40000));
  assert.deepEqual(pixel(p,10,10),merge([255,0,0,255],[255,255,255,255],20000));
  assert.deepEqual(pixel(p,45,45),merge([0,0,255,255],[255,255,255,255],40000));
 }});
}
{
 const q=overlapping();q.draws[0].brush.rgba=[220,17,40,113];q.draws[1].brush.rgba=[12,90,212,145];
 q.paths[0].commands[0].to=point(8n*U+U/3n,8n*U+U/2n);
 const transparent=structuredClone(q);delete transparent.opacityGroups;transparent.viewport.background=[0,0,0,0];
 const ref=native(transparent).pixels;
 requests.push({name:'aa-premultiplied',q,oracle:p=>{
  for(let i=0;i<p.length;i+=4)assert.deepEqual(Array.from(p.subarray(i,i+4)),merge(ref.subarray(i,i+4),[255,255,255,255],32768));
 }});
}
{
 const q=overlapping();q.draws.push(structuredClone(q.draws[0]));q.opacityGroups=[group(1,2)];
 q.clips=[{parent:null,path:0,origin:point(-12n*U)}];q.draws.forEach(d=>d.clip=0);
 requests.push({name:'clip-scope-restoration',q,oracle:p=>{assert.deepEqual(pixel(p,25,25),[255,0,0,255]);assert.deepEqual(pixel(p,29,25),[255,255,255,255]);}});
}
{
 const q=overlapping();q.draws.push({...structuredClone(q.draws[0]),brush:{kind:'snapshot',afterDraws:1},blend:'source'});q.opacityGroups=[group(0,3)];
 requests.push({name:'local-snapshot',q,oracle:p=>assert.deepEqual(pixel(p,25,25),[255,127,127,255])});
}
{
 const q=overlapping();q.draws[1].blend='source';q.draws[1].brush.rgba=[0,0,255,0];
 requests.push({name:'source-clear-inside-group',q,oracle:p=>assert.deepEqual(pixel(p,25,25),[255,255,255,255])});
}
{
 const q=overlapping();q.viewport.width=q.viewport.height=512;
 requests.push({name:'source/new/group-inherited-control',q});
}
for(const c of requests) {
 const n=native(c.q);assert.equal(JSON.parse(n.metadata).status,'rendered',n.metadata);
 const value=wasm.render_paths(JSON.stringify(c.q),backend);
 assert.equal(value.metadata,n.metadata,c.name);assert.deepEqual(Buffer.from(value.take_pixels()),n.pixels,c.name);
 c.oracle?.(n.pixels);
 const frame=Buffer.from(captured.buffer,captured.byteOffset,captured.byteLength);
 assert.equal(captured[1],13);
 const safe=c.name.replaceAll('/','-');
 const entry={name:c.name,status:0,frame:await save(safe+'.frame.bin',frame),currentPixels:await save(safe+'.rgba',n.pixels),
  request:await save(safe+'.json',Buffer.from(JSON.stringify(c.q))),metadata:await save(safe+'.result.json',Buffer.from(n.metadata))};
 cases.push(entry);
 // Independently use Draw IR; the interval survives geometry resource sharing.
 if(!c.q.clips?.length) {
  const scene={viewport:c.q.viewport,scene:{opacityGroups:c.q.opacityGroups,paths:c.q.paths,transforms:c.q.draws.map(d=>({parent:null,affine:{linear:[String(U),'0','0',String(U)],translation:d.origin}})),instances:c.q.draws.map((d,i)=>({...d,origin:undefined,transform:i}))}};
  const ns=native(scene,true),ws=wasm.render_scene(JSON.stringify(scene),backend);
  assert.equal(JSON.parse(ns.metadata).status,'rendered',ns.metadata);assert.equal(ws.metadata,ns.metadata);
  assert.deepEqual(Buffer.from(ws.take_pixels()),n.pixels);assert.deepEqual(ns.pixels,n.pixels);
 }
}
const invalid=[];
for(const [name,change] of [
 ['crossing',q=>{q.draws.push(q.draws[0]);q.opacityGroups=[group(0,2),group(1,3)];}],
 ['snapshot-escape',q=>{q.opacityGroups=[group(1,2)];q.draws[1].brush={kind:'snapshot',afterDraws:0};}],
 ['opacity-range',q=>q.opacityGroups[0].opacity=65536],
 ['depth',q=>q.opacityGroups=Array.from({length:65},()=>group(0,2))],
 ['memory',q=>{q.viewport.width=4096;q.viewport.height=4096;q.opacityGroups=[group(0,2),group(1,2)];}],
]) {
 const q=overlapping();change(q);const before=calls,n=native(q),w=wasm.render_paths(JSON.stringify(q),backend);
 assert.equal(w.metadata,n.metadata);assert.equal(JSON.parse(n.metadata).status,'error');assert.equal(calls,before);assert.equal(w.take_pixels().length,0);
 invalid.push({name,response:JSON.parse(n.metadata)});
}
// Exercise the component's independent grammar rather than trusting Rust admission.
const base=new Uint32Array(Uint8Array.from(await fs.readFile(cases[0].frame.path)).buffer);
const at=base.length-base[6]*8-base[14]*3;
for(const [name,index,value,status] of [['empty',at+1,0,1],['oversized',at+1,3,1],['opacity',at+2,65536,1],['count',14,4097,3],['unknown-version',1,15,1]]) {
 const f=base.slice();f[index]=value;
 const reply=component.raster(f);assert.equal(reply.status,status,name);assert.equal(reply.pixels.length,0);
 const bytes=Buffer.from(f.buffer),h=Buffer.alloc(4);h.writeUInt32LE(f.length);
 const n=spawnSync(probe,[],{input:Buffer.concat([h,bytes]),maxBuffer:1024*1024});assert.equal(n.status,0);assert.equal(n.stdout.readUInt32LE(),status);assert.equal(n.stdout.readUInt32LE(4),0);
 cases.push({name:'invalid/'+name,status,frame:await save('invalid-'+name+'.bin',bytes),currentPixels:await save('invalid-'+name+'.rgba',Buffer.alloc(0))});
}
const report={format:'musteroffice.opacity-group-parity/1',status:'passed',cases,invalid,wasmSha256:sha(wasmBytes),nativeWorkerSha256:sha(await fs.readFile(worker)),rustWasmSha256:sha(await fs.readFile(path.join(wasmPath,'mo_wasm_bg.wasm'))),scope:'Isolated raster groups; animation presets and product upgrade are separate.'};
await save('report.json',Buffer.from(JSON.stringify(report,null,2)+'\n'));
console.log(JSON.stringify({status:report.status,cases:cases.length,invalidRequests:invalid.length}));
