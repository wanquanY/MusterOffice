/** Edit real PPTX transforms; report exactness against independently permuted pixels. */
import fs from 'node:fs';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {createRequire} from 'node:module';import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/gradient-coordinates',out=root+'/render';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/transform-edit/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const raster=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const decoder={decodeImage(b){return raster.decodeImage(b);},invalidate(){raster.invalidate();}};
const priorPath='.codex-work/elliptic-source/source-parity.json',prior=JSON.parse(fs.readFileSync(priorPath));
const cases=[],baselineChanges=[];let comparedPixels=0,lastFrame=null;
const drawing={raster(f){lastFrame=f.slice();return raster.raster(f);},rasterImages(f,b){lastFrame=f.slice();return raster.rasterImages(f,b);},invalidate(){raster.invalidate();}};
function render(name,request,source){
 lastFrame=null;
 const json=JSON.stringify(request),w=wasm.render_pptx_resource_page(json,source,new Uint8Array(),decoder,shaper,drawing);
 const metadata=w.metadata,pixels=Buffer.from(w.take_pixels()),meta=JSON.parse(metadata);assert.equal(meta.status,'rendered',metadata);
 const header=Buffer.alloc(12);header.writeUInt32LE(Buffer.byteLength(json));header.writeUInt32LE(source.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',['--pptx-resource-page'],{input:Buffer.concat([header,Buffer.from(json),source]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);
 const m=n.stdout.readUInt32LE(),p=n.stdout.readUInt32LE(4);assert.equal(n.stdout.length,8+m+p);
 assert.equal(n.stdout.subarray(8,8+m).toString(),metadata);assert.deepEqual(n.stdout.subarray(8+m),pixels,name);
 assert(lastFrame);
 return {pixels,record:{response:put(out+'/'+name+'.response.json',metadata),pixels:put(out+'/'+name+'.rgba',pixels),frame:put(out+'/'+name+'.frame',Buffer.from(lastFrame.buffer))}};
}
for(const sourceName of ['center-point','off-center']){
 const c=prior.cases.find(c=>c.name==='new/'+sourceName),source=load(c.source),q=JSON.parse(load(c.request));assert(c);
 assert.equal(q.page.viewport.width,400);assert.equal(q.page.viewport.height,300);assert.equal(q.page.viewport.scale.denominator,4000);
 const before=render(sourceName+'-before',q,source);const oldPixels=load(c.pixels);let changedChannels=0,maxDelta=0;for(let i=0;i<oldPixels.length;i++){const d=Math.abs(oldPixels[i]-before.pixels[i]);changedChannels+=d!==0;maxDelta=Math.max(maxDelta,d);}baselineChanges.push({name:sourceName,oldPixels:c.pixels,currentPixels:before.record.pixels,changedChannels,maxDelta});assert.equal(q.page.viewport.origin.x,'0');
 const index=JSON.parse(wasm.inspect_pptx(source)).index,objects=index.surfaces[q.page.slide].objects;assert.equal(objects.length,1);
 const object=objects[0],t=object.transform;assert.deepEqual(t.origin,{x:'100000',y:'200000'});assert.deepEqual(t.size,{width:'1200000',height:'600000'});
 const fields=['origin','size','childOrigin','childSize','rotation','flipHorizontal','flipVertical'];
 const expected=Object.fromEntries(fields.map(k=>[k,t[k]??null]));
 for(const mode of ['translate','flip-h','flip-v','quarter']){
  const replacement=structuredClone(expected);let inverse;
  if(mode==='translate'){replacement.origin={x:'140000',y:'280000'};inverse=(x,y)=>[x-10,y-20];}
  if(mode==='flip-h'){replacement.flipHorizontal=true;inverse=(x,y)=>[349-x,y];}
  if(mode==='flip-v'){replacement.flipVertical=true;inverse=(x,y)=>[x,249-y];}
  if(mode==='quarter'){replacement.rotation=5400000;inverse=(x,y)=>[y+50,299-x];}
  const edit={expectedSourceSha256:c.source.sha256,edits:[{target:{part:q.page.slide,nativeId:object.nativeId},expected,replacement}]};
  const bytes=Buffer.from(wasm.edit_pptx_transforms(JSON.stringify(edit),source)),name=sourceName+'-'+mode;
  const changed=put(out+'/'+name+'.pptx',bytes),editRecord=put(out+'/'+name+'.edit.json',JSON.stringify(edit));
  const n=spawnSync('target/debug/mo-cli',['pptx-edit-transforms',editRecord.path,c.source.path,out+'/'+name+'.native.pptx'],{env:{},encoding:'utf8',timeout:60000});
  assert.equal(n.status,0,n.stderr);assert.deepEqual(fs.readFileSync(out+'/'+name+'.native.pptx'),bytes);
  const request=structuredClone(q);request.page.expectedSourceSha256=sha(bytes);
  const result=render(name,request,bytes),reference=Buffer.alloc(400*300*4,255);
  for(let y=0;y<300;y++)for(let x=0;x<400;x++){
   const [sx,sy]=inverse(x,y);if(sx>=0&&sx<400&&sy>=0&&sy<300)before.pixels.copy(reference,(y*400+x)*4,(sy*400+sx)*4,(sy*400+sx+1)*4);
  }
  let differentPixels=0,maxChannelDifference=0;const examples=[];
  for(let i=0;i<reference.length;i+=4){let changed=false;
   for(let k=0;k<4;k++){const d=Math.abs(result.pixels[i+k]-reference[i+k]);maxChannelDifference=Math.max(maxChannelDifference,d);changed||=d!==0;}
   if(changed){differentPixels++;if(examples.length<8)examples.push({x:(i/4)%400,y:Math.floor(i/1600),actual:[...result.pixels.subarray(i,i+4)],expected:[...reference.subarray(i,i+4)]});}
  }
  comparedPixels+=400*300;
  cases.push({name,source:c.source,edited:changed,edit:editRecord,request:put(out+'/'+name+'.request.json',JSON.stringify(request)),
   before:before.record,...result.record,reference:put(out+'/'+name+'.expected.rgba',reference),
   exactRigidTransform:differentPixels===0,differentPixels,maxChannelDifference,examples});
 }
}
fs.writeFileSync(root+'/render.json',JSON.stringify({format:'musteroffice.gradient-coordinate-render/1',cases,comparedPixels,
 baselineChanges,beforeNativeWasmPages:2,afterNativeWasmPages:8,exactCases:cases.filter(c=>c.exactRigidTransform).length,
 discrepancyCases:cases.filter(c=>!c.exactRigidTransform).length,rigidTransformInvarianceAchieved:cases.every(c=>c.exactRigidTransform),prior:entry(priorPath),
 currentArtifacts:[entry('target/debug/mo-raster-worker'),entry('.codex-work/transform-edit/wasm-node/mo_wasm_bg.wasm'),entry('.codex-work/gradient-coordinates/component/mo-skia.wasm')],
 scope:'Two native gradient pages and actual source transform edits. Current before-images independently permuted for exact rigid-transform checks. Previous pixels preserved as observed deltas, not a correctness oracle for corrected mapping. Not target application acceptance.'},null,2)+'\n');
console.log(JSON.stringify({editedPages:cases.length,comparedPixels,exactCases:cases.filter(c=>c.exactRigidTransform).length,
 discrepancies:cases.filter(c=>!c.exactRigidTransform).map(c=>({name:c.name,pixels:c.differentPixels,maxChannelDifference:c.maxChannelDifference}))}));
assert(cases.every(c=>c.exactRigidTransform),'rigid transform precision must be exact');
