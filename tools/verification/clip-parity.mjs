import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import factory from '../../.codex-work/clips/component/mo-skia.mjs';
import {fixtures} from './clip-fixtures.mjs';
const root='.codex-work/clips',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
export const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const module=new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm'));
const raster=await RasterComponent.create(factory,module);assert(raster.supportsClips&&raster.supportsImageDomains);
const wasm=createRequire(import.meta.url)('../../.codex-work/clips/wasm-node/mo_wasm.js');
const images=Buffer.from(Array.from({length:20},(_,i)=>[30+(i%5)*40,20+Math.floor(i/5)*55,160,255]).flat());
const resources=[{width:5,height:4,alpha:'premultiplied',sha256:createHash('sha256').update(images).digest('hex')}];
put(out+'/images.rgba',images);
let calls=0,frame;
const backend={raster(f){calls++;frame=f.slice();return raster.raster(f);},rasterImages(f,b){calls++;frame=f.slice();return raster.rasterImages(f,b);},invalidate(){raster.invalidate();}};
let paired=0;
function run(q,scene,image){
 const request=JSON.stringify(image?{raster:q,images:resources}:q),data=Buffer.from(request),bytes=image?images:Buffer.alloc(0),h=Buffer.alloc(image?8:4);h.writeUInt32LE(data.length);if(image)h.writeUInt32LE(bytes.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',image?[scene?'--image-scene':'--images']:scene?['--scene']:[],{input:Buffer.concat([h,data,bytes]),env:{},timeout:60000,maxBuffer:90*1024*1024});
 assert.equal(n.status,0,n.stderr.toString());const size=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+size).toString(),pixels=n.stdout.subarray(8+size);assert.equal(pixels.length,n.stdout.readUInt32LE(4));calls=0;frame=null;
 const w=image?(scene?wasm.render_image_scene(request,bytes,backend):wasm.render_image_paths(request,bytes,backend)):(scene?wasm.render_scene(request,backend):wasm.render_paths(request,backend));
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!raster.invalid);paired++;
 return {request,metadata,pixels,response:JSON.parse(metadata),calls,frame};
}
const records=[];let referencePixels=0,maxDeviation=0;
for(const c of fixtures()){
 const p=run(c.request,false,c.images),s=run(c.scene,true,c.images);
 assert.equal(p.response.status,'rendered',c.name+': '+p.metadata);assert.equal(s.response.status,'rendered',c.name+': '+s.metadata);
 assert.equal(p.calls,1);assert.equal(s.calls,1);assert.equal(p.frame[1],7);assert.equal(s.frame[1],7);assert.deepEqual(s.pixels,p.pixels,c.name);
 let delta=null;
 if(c.mask){
  const expected=Buffer.alloc(p.pixels.length);
  for(const draw of c.request.draws){
   const layer=run({...c.request,clips:[],draws:[{...draw,clip:null}]},false,c.images);assert.equal(layer.response.status,'rendered');
   for(let y=0;y<24;y++)for(let x=0;x<32;x++)if(c.mask(draw,x+0.5,y+0.5)){
    const i=(y*32+x)*4,alpha=layer.pixels[i+3];
    for(let k=0;k<4;k++)expected[i+k]=layer.pixels[i+k]+Math.floor((expected[i+k]*(255-alpha)+127)/255);
   }
  }
  delta=0;for(let i=0;i<expected.length;i++)delta=Math.max(delta,Math.abs(expected[i]-p.pixels[i]));
  assert(delta<=1,`${c.name}: mask/composition max deviation ${delta}`);maxDeviation=Math.max(maxDeviation,delta);referencePixels+=32*24;
  put(out+'/'+c.name+'.expected.rgba',expected);
 }
 const path=out+'/'+c.name;
 records.push({name:c.name,images:c.images,request:put(path+'.path.json',p.request),sceneRequest:put(path+'.scene.json',s.request),pathResponse:put(path+'.path-response.json',p.metadata),sceneResponse:put(path+'.scene-response.json',s.metadata),frame:put(path+'.frame',Buffer.from(p.frame.buffer)),sceneFrame:put(path+'.scene-frame',Buffer.from(s.frame.buffer)),pixels:put(path+'.rgba',p.pixels),maxChannelDeviation:delta});
}
const negatives=[];
for(const [name,mutate] of [
 ['draw-reference',q=>q.draws[0].clip=999],['clip-path',q=>q.clips[0].path=999],['parent-self',q=>q.clips[0].parent=0],
 ['range',q=>q.clips[0].origin.x=String(32768n<<32n)],['unknown-field',q=>q.clips[0].stroke={}],
 ['depth',q=>{q.clips=Array.from({length:65},(_,i)=>({...q.clips[0],parent:i?i-1:null}));q.draws[0].clip=64;}],
]){
 const q=structuredClone(fixtures()[0].request);mutate(q);const r=run(q,false,false);assert.equal(r.response.status,'error',name);assert.equal(r.calls,0);assert.equal(r.pixels.length,0);
 negatives.push({name,request:put(out+'/'+name+'.invalid.json',r.request),response:put(out+'/'+name+'.invalid-response.json',r.metadata)});
}
const cliChecks=[];
for(const [command,key,responseKey] of [['render-paths','request','pathResponse'],['render-scene','sceneRequest','sceneResponse']]){
 const c=records[0],path=out+'/'+command+'.cli.rgba';fs.rmSync(path,{force:true});
 const execute=()=>spawnSync('target/debug/mo-cli',[command,c[key].path,path],{env:{},timeout:60000,encoding:'utf8',maxBuffer:90*1024*1024});
 const n=execute();assert.equal(n.status,0,n.stderr);assert.equal(n.stdout.trimEnd(),fs.readFileSync(c[responseKey].path,'utf8'));assert.deepEqual(fs.readFileSync(path),fs.readFileSync(c.pixels.path));
 assert.notEqual(execute().status,0);assert.deepEqual(fs.readFileSync(path),fs.readFileSync(c.pixels.path));
 cliChecks.push({command,exclusivePublication:true,pixels:entry(path)});
}
const result={format:'musteroffice.shared-clips-parity/1',cliChecks,cli:entry('target/debug/mo-cli'),pairedCalls:paired,referencePixels,maxChannelDeviation:maxDeviation,cases:records,negatives,component:entry(root+'/component/mo-skia.wasm'),worker:entry('target/debug/mo-raster-worker'),rustWasm:entry(root+'/wasm-node/mo_wasm_bg.wasm'),adapter:entry(root+'/ts-raster/index.js'),images:entry(out+'/images.rgba')};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({cases:records.length,paired,referencePixels,maxDeviation,preflightFailures:negatives.length}));
