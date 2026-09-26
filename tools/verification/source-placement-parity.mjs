import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root='.codex-work/source-placement',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),cases=[];
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
let frame;
const backend={raster(words){frame=Buffer.alloc(words.length*4);words.forEach((w,i)=>frame.writeUInt32LE(w,i*4));return component.raster(words);},invalidate(){component.invalidate();}};
function store(name,kind,q,raw,source,validRequest=true){
 const r={name,kind,sourcePath:source.path,sourceSha256:source.sha256,responsePath:`${root}/${name}.${kind}.response.json`,responseSha256:sha(raw),validRequest};fs.writeFileSync(r.responsePath,raw);
 if(q!==null){const s=typeof q==='string'?q:JSON.stringify(q);r.requestPath=`${root}/${name}.${kind}.request.json`;r.requestSha256=sha(s);fs.writeFileSync(r.requestPath,s);}
 cases.push(r);return r;
}
function query(c,q,kind='placement',validRequest=true){
 const raw=typeof q==='string'?q:JSON.stringify(q),path=root+'/query.json';fs.writeFileSync(path,raw);
 const command=kind==='placement'?'pptx-placements':'pptx-paths',fn=kind==='placement'?'place_pptx_objects':'compile_pptx_paths';
 const n=spawnSync('target/release/mo-cli',[command,path,c.path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(n.status,0,n.stderr);
 const w=wasm[fn](raw,fs.readFileSync(c.path));assert.equal(n.stdout.trimEnd(),w,c.name);store(c.name,kind,q,w,c,validRequest);return JSON.parse(w);
}
function render(c,placement,paths){
 // Geometry and placement are real source results. The deliberately explicit
 // red diagnostic paint does not claim native fill/effect/page composition.
 const q={viewport:{width:800,height:450,origin:{x:'0',y:'0'},scale:{numerator:800,denominator:12192000},coordinateTolerance:'16777216',background:[255,255,255,255]},scene:{paths:[],transforms:[{parent:null,affine:placement.affine}],instances:[]}};
 for(const p of paths){
  const commands=p.commands.map(command=>{const out={kind:command.kind};for(const [key,point]of Object.entries(command))if(key!=='kind')out[key]={x:String(BigInt(point.x)-BigInt(placement.anchor.x)),y:String(BigInt(point.y)-BigInt(placement.anchor.y))};return out;});
  q.scene.instances.push({path:q.scene.paths.length,transform:0,brush:{kind:'solid',rgba:[219,35,75,255]}});q.scene.paths.push({fillRule:'nonzero',commands});
 }
 const raw=JSON.stringify(q),h=Buffer.alloc(4);h.writeUInt32LE(Buffer.byteLength(raw));
 const n=spawnSync('target/release/mo-raster-worker',['--scene'],{input:Buffer.concat([h,Buffer.from(raw)]),timeout:30000,maxBuffer:80*1024*1024,env:{}});assert.equal(n.status,0,n.stderr?.toString());
 const m=n.stdout.readUInt32LE(),meta=n.stdout.subarray(8,8+m).toString(),pixels=n.stdout.subarray(8+m);assert.equal(n.stdout.readUInt32LE(4),pixels.length);
 frame=null;const w=wasm.render_scene(raw,backend);assert.equal(w.metadata,meta);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert.equal(JSON.parse(meta).status,'rendered',meta);
 const r=store(c.name,'raster',q,meta,c);Object.assign(r,{pixelsPath:`${root}/${c.name}.rgba`,pixelsSha256:sha(pixels),framePath:`${root}/${c.name}.frame.bin`,frameSha256:sha(frame)});fs.writeFileSync(r.pixelsPath,pixels);fs.writeFileSync(r.framePath,frame);
}
const manifest=JSON.parse(fs.readFileSync(root+'/manifest.json'));
for(const c of manifest.cases){
 const bytes=fs.readFileSync(c.path);assert.equal(sha(bytes),c.sha256);
 const n=spawnSync('target/release/mo-cli',['pptx-inspect',c.path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(n.status,0,n.stderr);
 const s=wasm.inspect_pptx(bytes);assert.equal(n.stdout.trimEnd(),s,c.name);store(c.name,'source',null,s,c);
 const q={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',objects:c.ids,profile:'drawingml-source-sector-scale-q96-v1-draft'};
 const r=query(c,q);
 if(c.error){assert.equal(r.status,'error');assert.equal(r.error.code,c.error,c.name);continue;}
 assert.equal(r.status,'evaluated',c.name+JSON.stringify(r));assert.deepEqual(r.placements.objects.map(o=>o.nativeId),c.ids);assert.equal(r.placements.rootTransformIgnored,c.root!==null);
 for(const o of r.placements.objects){if(c.unresolved&&(!c.unresolvedIds||c.unresolvedIds.includes(o.nativeId))){assert.equal(o.outcome.status,'unresolved',c.name);assert.equal(o.outcome.reason.cause.kind,c.unresolved,c.name);}else assert.equal(o.outcome.status,'resolved',c.name+JSON.stringify(o));}
 if(c.render&&!c.unresolved){const p=query(c,{geometry:{expectedSourceSha256:c.sha256,surface:q.surface,objects:[99],profile:'ecma376-2016-ms-presets-draft-v2'},options:{profile:'drawingml-polar-arcs-q96-hermite-v1-draft',coordinateTolerance:'4294967296'}},'paths');assert.equal(p.paths.objects[0].outcome.status,'compiled');render(c,r.placements.objects[0].outcome.placement,p.paths.objects[0].outcome.paths);}
}
const c=manifest.cases[0],base={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',objects:[99],profile:'drawingml-source-sector-scale-q96-v1-draft'};
for(const [name,change,code,valid]of [
 ['conflict',q=>q.expectedSourceSha256='0'.repeat(64),'SOURCE_CONFLICT',true],['missing-object',q=>q.objects=[0],'INPUT_INVALID',true],['missing-surface',q=>q.surface='/ppt/no.xml','INPUT_INVALID',true],['query-budget',q=>q.objects=Array(257).fill(99),'LIMIT_EXCEEDED',true],['unknown-profile',q=>q.profile='future','INPUT_INVALID',false],['numeric-id',q=>q.objects=['99'],'INPUT_INVALID',false],['unknown-field',q=>q.script='x','INPUT_INVALID',false]]){
 const q=structuredClone(base);change(q);const r=query({...c,name},q,'placement',valid);assert.equal(r.error.code,code,name);
}
for(const [name,objects]of [['duplicates',[99,99,99]],['empty',[]]]){const r=query({...c,name},{...base,objects});assert.deepEqual(r.placements.objects.map(o=>o.nativeId),objects);}
const bad=JSON.stringify(base).replace('"objects":','"objects":[],"objects":');assert.equal(query({...c,name:'duplicate-key'},bad,'placement',false).error.code,'INPUT_INVALID');
const report={format:'musteroffice.source-placement-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),nativeRasterWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),exactNativeWasmResponsesAndPixels:true,cases};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,rendered:cases.filter(c=>c.kind==='raster').length}));
