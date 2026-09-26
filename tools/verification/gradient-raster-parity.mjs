import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import {fixtures} from './gradient-raster-fixtures.mjs';
const root='.codex-work/gradient-raster',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
fs.mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex'),componentBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm');
const component=await RasterComponent.create(factory,new WebAssembly.Module(componentBytes)),cases=[],frames=new Map();let frame,calls=0;
const backend={raster(words){calls++;frame=Buffer.alloc(words.length*4);words.forEach((w,i)=>frame.writeUInt32LE(w,i*4));return component.raster(words);},invalidate(){component.invalidate();}};
function packet(json){const data=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(data.length);return Buffer.concat([h,data]);}
function probe(bytes,sanitized=false){const h=Buffer.alloc(4);h.writeUInt32LE(bytes.length/4);const r=spawnSync('.codex-work/skia/mo-skia-probe'+(sanitized?'-asan':''),[],{input:Buffer.concat([h,bytes]),timeout:30000,maxBuffer:80*1024*1024,env:{...process.env,ASAN_OPTIONS:'detect_leaks=0:halt_on_error=1',UBSAN_OPTIONS:'halt_on_error=1'}});assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.readUInt32LE(4),r.stdout.length-8);return {status:r.stdout.readUInt32LE(),pixels:r.stdout.subarray(8)};}
for(const c of fixtures()){
 const json=JSON.stringify(c.q),n=spawnSync('target/release/mo-raster-worker',c.scene?['--scene']:[],{input:packet(json),timeout:30000,maxBuffer:80*1024*1024,env:{}});
 assert.equal(n.status,0,n.stderr?.toString());const m=n.stdout.readUInt32LE(),len=n.stdout.readUInt32LE(4),metadata=n.stdout.subarray(8,8+m).toString(),pixels=n.stdout.subarray(8+m);assert.equal(pixels.length,len);
 frame=null;const before=calls,w=(c.scene?wasm.render_scene:wasm.render_paths)(json,backend);
 assert.equal(w.metadata,metadata,c.name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,c.name);
 const response=JSON.parse(metadata),status=response.status==='error'?response.error.code:response.status;assert.equal(status,c.expected??'rendered',c.name);
 const rec={name:c.name,scene:c.scene??false,oracle:c.oracle??false,validRequest:c.validRequest??true,status,requestPath:`${root}/${c.name}.request.json`,responsePath:`${root}/${c.name}.response.json`,requestSha256:sha(json),responseSha256:sha(metadata)};
 fs.writeFileSync(rec.requestPath,json);fs.writeFileSync(rec.responsePath,metadata);
 if(status==='rendered'){
  assert.equal(calls,before+1);const info=c.scene?response.info.raster:response.info;
  assert.equal(sha(frame),info.frameSha256);assert.equal(sha(pixels),info.sha256);
  const n=probe(frame),a=probe(frame,true);assert.equal(n.status,0);assert.equal(a.status,0);assert.deepEqual(n.pixels,pixels);assert.deepEqual(a.pixels,pixels);
  Object.assign(rec,{framePath:`${root}/${c.name}.frame.bin`,pixelsPath:`${root}/${c.name}.rgba`,frameSha256:sha(frame),pixelsSha256:sha(pixels),work:info.work});
  fs.writeFileSync(rec.framePath,frame);fs.writeFileSync(rec.pixelsPath,pixels);frames.set(c.name,frame);
  if(c.equivalent){assert.deepEqual(frame,frames.get(c.equivalent));rec.equivalent=c.equivalent;}
 }else {assert.equal(calls,before);assert.equal(pixels.length,0);assert.equal(frame,null);}
 cases.push(rec);
}
// Forge the component wire directly, so C++ cannot rely on Rust validation.
const good=frames.get('linear-clamp-srgb-straight'),start=10+2+5*7,words=Array.from({length:good.length/4},(_,i)=>good.readUInt32LE(i*4));
const mutations=[['old-abi',1,3,1],['gradient-count',9,4097,3],['geometry',start,2,1],['tile',start+1,4,1],['space',start+2,2,1],['alpha',start+3,2,1],['few-stops',start+4,1,1],['many-stops',start+4,4097,3],['nan-coordinate',start+5,0x7fc00000,1],['unordered-stop',start+9+5,0xbf800000,1],['nan-color',start+10,0x7fc00000,1],['invalid-alpha',start+13,0x40000000,1],['brush-reference',words.length-1,2,1],['ambiguous-color',words.length-3,1,1]];
const rejected=[];
for(const [name,i,value,status] of mutations){const raw=Buffer.from(good);raw.writeUInt32LE(value,i*4);const n=probe(raw),a=probe(raw,true),w=component.raster(Uint32Array.from({length:raw.length/4},(_,j)=>raw.readUInt32LE(j*4)));assert.equal(n.status,status,name);assert.equal(a.status,status,name);assert.equal(w.status,status,name);assert.equal(n.pixels.length+a.pixels.length+w.pixels.length,0);rejected.push({name,status,sha256:sha(raw)});}
const report={format:'musteroffice.gradient-raster-parity/1',nativeWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),adapterSha256:sha(fs.readFileSync('.codex-work/raster-component/index.js')),nativeProbeSha256:sha(fs.readFileSync('.codex-work/skia/mo-skia-probe')),sanitizedProbeSha256:sha(fs.readFileSync('.codex-work/skia/mo-skia-probe-asan')),cases,forgedComponentRejections:rejected,exactNativeWasmPixelsAndMetadata:true,sanitizedPixelsIdentical:true};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,rendered:cases.filter(c=>c.status==='rendered').length,forgedRejections:rejected.length}));
