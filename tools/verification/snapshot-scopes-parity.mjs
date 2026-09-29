/** Explicit immutable capture ownership, independent pixel oracles and ABI refusal. */
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {solid, point, U} from './path-raster-fixtures.mjs';
const [out, worker, wasmPath, componentPath, tsPath, oldPath, probe] = process.argv.slice(2);
assert(out && worker && wasmPath && componentPath && tsPath && oldPath && probe);
await fs.mkdir(out, {recursive:false});
const sha = b => createHash('sha256').update(b).digest('hex');
async function save(name, bytes) {
  const file=path.join(out,name);await fs.writeFile(file,bytes,{flag:'wx'});
  return {path:file,byteLength:bytes.length,sha256:sha(bytes)};
}
const {RasterComponent}=await import(pathToFileURL(path.resolve(tsPath,'index.js')));
async function load(p) {
  const {default:factory}=await import(pathToFileURL(path.resolve(p,'mo-skia.mjs')));
  const bytes=await fs.readFile(path.join(p,'mo-skia.wasm'));
  return [await RasterComponent.create(factory,await WebAssembly.compile(bytes)),sha(bytes)];
}
const [component,componentSha]=await load(componentPath),[old,oldSha]=await load(oldPath);
assert(component.supportsSnapshotScopes && !old.supportsSnapshotScopes);
const wasm=createRequire(import.meta.url)(path.resolve(wasmPath,'mo_wasm.js'));
const native=q=>{
  const b=Buffer.from(JSON.stringify(q)),h=Buffer.alloc(4);h.writeUInt32LE(b.length);
  const r=spawnSync(worker,[],{input:Buffer.concat([h,b]),env:{},timeout:30000,maxBuffer:80*1024*1024});
  assert.equal(r.status,0,r.stderr?.toString());const n=r.stdout.readUInt32LE(),size=r.stdout.readUInt32LE(4);
  assert.equal(r.stdout.length,8+n+size);
  return {metadata:r.stdout.subarray(8,8+n).toString(),pixels:r.stdout.subarray(8+n)};
};
let captured,calls=0;
const backend={raster(frame){calls++;captured=frame.slice();return component.raster(frame);},invalidate(){component.invalidate();}};
const bg=[11,31,61,255],red=[255,0,0,255],green=[0,255,0,255],blue=[0,0,255,255];
const g=(firstDraw,endDraw,opacity=32768)=>({firstDraw,endDraw,opacity});
const paint=rgba=>({path:0,origin:point(),brush:{kind:'solid',rgba}});
const snap=(afterDraws,scope)=>({path:0,origin:point(),brush:{kind:'snapshot',afterDraws,scope},blend:'source'});
const root={kind:'output'},group=index=>({kind:'group',index});
const merge=(s,d,a)=>{const v=s.map(x=>Math.round(x*a/65535));return v.map((x,k)=>x+Math.round(d[k]*(255-v[3])/255));};
const cases=[],requests=[];
function add(name,draws,opacityGroups,inside,width=64) {
  const q=solid();q.viewport.background=bg;q.viewport.width=q.viewport.height=width;q.draws=draws;q.opacityGroups=opacityGroups;
  requests.push({name,q,inside});
}
add('output-before-root-paint',[paint(red),paint(blue),snap(0,root)],[g(1,3)],merge(bg,red,32768));
add('output-at-opening-retained-after-close',[paint(red),paint(green),paint(blue),snap(1,root)],[g(1,2),g(3,4)],merge(red,blue,32768));
add('outer-capture-at-inner-opening',[paint(red),paint(blue),snap(1,group(0)),snap(1,group(0))],[g(0,3),g(1,2)],red);
add('distinct-canvases-same-prefix',[paint(red),paint(green),snap(2,root),snap(2,group(0))],[g(1,4),g(2,4)],merge(green,red,32768));
add('capture-after-closing-merge',[paint(red),paint(blue),snap(1,root)],[g(0,1),g(2,3)],merge(merge(red,bg,32768),blue,32768));
add('capture-root-at-first-group-opening',[paint(red),paint(blue),snap(0,root)],[g(0,3)],bg);
add('large-scoped-copy',[paint(red),paint(green),paint(blue),snap(1,root)],[g(1,2),g(3,4)],merge(red,blue,32768),512);
for(const c of requests) {
  const n=native(c.q);assert.equal(JSON.parse(n.metadata).status,'rendered',n.metadata);
  const w=wasm.render_paths(JSON.stringify(c.q),backend);assert.equal(w.metadata,n.metadata,c.name);
  assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels,c.name);assert.equal(captured[1],14);
  // All paths are the same integer-aligned rectangle. Every pixel is checked
  // against independently specified color compositions, including outside it.
  const width=c.q.viewport.width;
  for(let y=0;y<width;y++)for(let x=0;x<width;x++) {
    const at=(y*width+x)*4,expected=x>=8&&x<40&&y>=8&&y<40?c.inside:bg;
    assert.deepEqual(Array.from(n.pixels.subarray(at,at+4)),expected,`${c.name}/${x}/${y}`);
  }
  assert.throws(()=>old.raster(captured),/snapshot scope extension/);
  assert.throws(()=>old.beginRaster(captured),/snapshot scope extension/);
  const bytes=Buffer.from(captured.buffer);
  cases.push({name:c.name,status:0,frame:await save(c.name+'.bin',bytes),currentPixels:await save(c.name+'.rgba',n.pixels),request:await save(c.name+'.json',Buffer.from(JSON.stringify(c.q))),metadata:await save(c.name+'.metadata.json',Buffer.from(n.metadata))});
  // Draw IR owns the same capture/group identities through geometry lowering.
  const scene={viewport:c.q.viewport,scene:{opacityGroups:c.q.opacityGroups,paths:c.q.paths,transforms:c.q.draws.map(d=>({parent:null,affine:{linear:[String(U),'0','0',String(U)],translation:d.origin}})),instances:c.q.draws.map((d,i)=>({...d,origin:undefined,transform:i}))}};
  const result=wasm.render_scene(JSON.stringify(scene),backend);
  assert.equal(JSON.parse(result.metadata).status,'rendered');assert.deepEqual(Buffer.from(result.take_pixels()),n.pixels);
}
const rejected=[];
for(const [name,change] of [
  ['unknown-group',q=>q.draws[2].brush.scope=group(999)],
  ['not-yet-started',q=>{q.draws[2].brush.scope=group(0);q.draws[2].brush.afterDraws=0;}],
  ['already-closed',q=>{q.opacityGroups[0].endDraw=2;q.draws[2].brush.scope=group(0);q.draws[2].brush.afterDraws=2;}],
  ['future',q=>q.draws[2].brush.afterDraws=3],
  ['legacy-escape',q=>delete q.draws[2].brush.scope],
  ['shared-pixel-budget',q=>{q.viewport.width=8192;q.viewport.height=2048;}],
]) {
  const q=structuredClone(requests[0].q);change(q);const count=calls,n=native(q),w=wasm.render_paths(JSON.stringify(q),backend);
  assert.equal(w.metadata,n.metadata);assert.equal(JSON.parse(n.metadata).status,'error');assert.equal(w.take_pixels().length,0);assert.equal(calls,count);
  rejected.push({name,response:JSON.parse(n.metadata)});
}
const base=new Uint32Array(Uint8Array.from(await fs.readFile(cases[0].frame.path)).buffer);
const at=base.length-base[6]*8-base[14]*3-base[13]*2;
for(const [name,changes,status] of [
  ['unknown-scope',[[at+1,2]],1],['inactive-source',[[at+1,1]],1],
  ['future-prefix',[[at,base[6]]],1],['unknown-version',[[1,15]],1],
]) {
  const frame=base.slice();for(const [i,v] of changes)frame[i]=v;
  const r=component.raster(frame);assert.equal(r.status,status,name);assert.equal(r.pixels.length,0);
  const b=Buffer.from(frame.buffer),h=Buffer.alloc(4);h.writeUInt32LE(frame.length);
  const n=spawnSync(probe,[],{input:Buffer.concat([h,b]),timeout:30000,maxBuffer:1048576});
  assert.equal(n.status,0);assert.equal(n.stdout.readUInt32LE(),status);assert.equal(n.stdout.readUInt32LE(4),0);
  cases.push({name:'invalid/'+name,status,frame:await save('invalid-'+name+'.bin',b),currentPixels:await save('invalid-'+name+'.rgba',Buffer.alloc(0))});
}
await save('report.json',Buffer.from(JSON.stringify({format:'musteroffice.snapshot-scopes-parity/1',status:'passed',componentSha,oldSha,cases,rejected,nativeWorkerSha256:sha(await fs.readFile(worker)),rustWasmSha256:sha(await fs.readFile(path.join(wasmPath,'mo_wasm_bg.wasm')))},null,2)+'\n'));
console.log(JSON.stringify({status:'passed',valid:requests.length,invalidJson:rejected.length,invalidBinary:cases.length-requests.length}));
