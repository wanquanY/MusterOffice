import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root='.codex-work/preset-expansion',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),records=[];
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
let frame;
const backend={raster(words){frame=Buffer.alloc(words.length*4);words.forEach((w,i)=>frame.writeUInt32LE(w,i*4));return component.raster(words);},invalidate(){component.invalidate();}};
function save(name,kind,q,value,source){
 const request=JSON.stringify(q),requestPath=`${root}/${name}.${kind}.request.json`,responsePath=`${root}/${name}.${kind}.response.json`;
 fs.writeFileSync(requestPath,request);fs.writeFileSync(responsePath,value);
 const record={name,kind,sourcePath:source.path,sourceSha256:source.sha256,requestPath,responsePath,requestSha256:sha(request),responseSha256:sha(value)};records.push(record);return record;
}
function run(kind,c,q){
 const raw=JSON.stringify(q),temp=`${root}/current-query.json`;fs.writeFileSync(temp,raw);
 const n=spawnSync('target/release/mo-cli',[kind==='geometry'?'pptx-geometry':'pptx-paths',temp,c.path],{encoding:'utf8',timeout:120000,maxBuffer:128*1024*1024});assert.equal(n.status,0,n.stderr);
 const w=wasm[kind==='geometry'?'evaluate_pptx_geometry':'compile_pptx_paths'](raw,fs.readFileSync(c.path));assert.equal(n.stdout.trimEnd(),w,c.name+'/'+kind);
 save(c.name,kind,q,w,c);return JSON.parse(w);
}
function render(c,paths){
 const U=1n<<32n,point=(x=0n,y=0n)=>({x:String(x),y:String(y)}),q={viewport:{width:96,height:96,origin:point(),scale:{numerator:80,denominator:1000001},coordinateTolerance:String(1<<24),background:[255,255,255,255]},paths:paths.map(p=>({fillRule:'nonzero',commands:p.commands})),draws:[]};
 // Deliberately explicit diagnostic paint. Native slide fill/line resolution is
 // a separate stage; these pixels verify actual preset geometry and backend use.
 for(const [i,p] of paths.entries()){
  const draw={path:i,origin:point(1000001n*U/10n,1000001n*U/10n),brush:{kind:'solid',rgba:[55,132,201,255]}};
  if(p.fill!=='none')q.draws.push(draw);
  if(p.stroke!==false)q.draws.push({...draw,brush:{kind:'solid',rgba:[20,43,64,255]},stroke:{width:String(1000001n*U/80n),cap:'butt',join:{kind:'round'}}});
 }
 const raw=JSON.stringify(q),h=Buffer.alloc(4);h.writeUInt32LE(Buffer.byteLength(raw));
 const n=spawnSync('target/release/mo-raster-worker',[],{input:Buffer.concat([h,Buffer.from(raw)]),timeout:30000,maxBuffer:80*1024*1024,env:{}});assert.equal(n.status,0,n.stderr?.toString());
 const len=n.stdout.readUInt32LE(),meta=n.stdout.subarray(8,8+len).toString(),pixels=n.stdout.subarray(8+len);assert.equal(n.stdout.readUInt32LE(4),pixels.length);
 frame=null;const w=wasm.render_paths(raw,backend);assert.equal(w.metadata,meta,c.name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,c.name);assert.equal(JSON.parse(meta).status,'rendered',meta);
 const r=save(c.name,'raster',q,meta,c);Object.assign(r,{pixelsPath:`${root}/${c.name}.rgba`,pixelsSha256:sha(pixels),framePath:`${root}/${c.name}.frame.bin`,frameSha256:sha(frame)});fs.writeFileSync(r.pixelsPath,pixels);fs.writeFileSync(r.framePath,frame);
}
const manifest=JSON.parse(fs.readFileSync(root+'/manifest.json'));
for(const [i,c] of manifest.cases.entries()){
 assert.equal(sha(fs.readFileSync(c.path)),c.sha256);
 const q={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',objects:[c.objectId],profile:'ecma376-2016-ms-presets-draft-v2'};
 const g=run('geometry',c,q);assert.equal(g.status,'evaluated');const go=g.geometry.objects[0].outcome;
 const p=run('paths',c,{geometry:q,options:{profile:'drawingml-polar-arcs-q96-hermite-v1-draft',coordinateTolerance:'4294967296'}});assert.equal(p.status,'compiled');const po=p.paths.objects[0].outcome;
 if(c.error){assert.equal(go.status,'unresolved');assert.equal(go.reason.kind,c.error);assert.equal(go.reason.origin.kind,'document');assert.deepEqual(po.reason,go.reason);}
 else {assert.equal(go.status,'resolved',c.name+JSON.stringify(go));assert.equal(po.status,'compiled',c.name+JSON.stringify(po));assert.deepEqual(po.paths.map(p=>p.origin),go.geometry.paths.map(p=>p.origin));if(c.render)render(c,po.paths);}
 if(i%50===0)console.log(JSON.stringify({completed:i+1,total:manifest.cases.length}));
}
const report={format:'musteroffice.preset-expansion-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),nativeRasterWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),cases:records,exactNativeWasmResponsesAndPixels:true};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:records.length,rendered:records.filter(c=>c.kind==='raster').length}));
