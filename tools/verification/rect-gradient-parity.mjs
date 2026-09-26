/** Rectangular fields: public Native/WASM and an independent binary64 pixel oracle. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/rect-gradient/ts-raster/index.js';
import factory from '../../.codex-work/rect-gradient/component/mo-skia.mjs';
import {scene} from './compositing-fixtures.mjs';
const root='.codex-work/rect-gradient',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
assert(component.supportsRectGradients);
const wasm=createRequire(import.meta.url)('../../.codex-work/rect-gradient/wasm-node/mo_wasm.js');
const fixed=n=>String(BigInt(Math.round(n*4294967296))),point=(x,y)=>({x:fixed(x),y:fixed(y)});
let frame,calls=0,paired=0;
const backend={raster(f){frame=f.slice();calls++;return component.raster(f);},invalidate(){component.invalidate();}};
function run(q,s=false){
 const json=JSON.stringify(q),b=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(b.length);
 const n=spawnSync('target/debug/mo-raster-worker',s?['--scene']:[],{input:Buffer.concat([h,b]),env:{},timeout:60000,maxBuffer:1<<26});
 assert.equal(n.status,0,n.stderr.toString());const ml=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+ml).toString(),pixels=n.stdout.subarray(8+ml);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 frame=null;calls=0;const w=s?wasm.render_scene(json,backend):wasm.render_paths(json,backend);
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!component.invalid);paired++;
 return {json,metadata,pixels,response:JSON.parse(metadata),frame,calls};
}
const tiling=(v,t)=>t==='repeat'?v-Math.floor(v):t==='mirror'?1-Math.abs(((v%2+2)%2)-1):Math.max(0,Math.min(1,v));
const planes=[['mirror',[32,-8],[128,0],[0,32],'mirror','mirror'],['reflected',[224,-8],[-128,0],[0,32],'repeat','clamp'],['shear',[32,-8],[128,32],[32,64],'clamp','mirror']];
const fields=[['point',[2,2,2,2]],['area-outset',[4,2,1.5,0]],['whole',[0,0,0,0]],['three-percent',[100/3,100/3,100/3,100/3]],['tiny',[1e12,2,2,2]],['wide-rate',[2**64,4,4,4]]];
const colors=[[.1,.8,.4,1],[.9,.2,.6,1]];
function request(plane,rates,interpolation,alpha=false){
 const [,origin,xStep,yStep,tileX,tileY]=plane;
 return {clips:[],viewport:{width:257,height:41,origin:point(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:fixed(1/256),background:[0,0,0,0]},paths:[{fillRule:'nonzero',commands:[{kind:'move',to:point(0,0)},{kind:'line',to:point(257,0)},{kind:'line',to:point(257,41)},{kind:'line',to:point(0,41)},{kind:'close'}]}],draws:[{path:0,origin:point(0,0),brush:{kind:'gradient',gradient:{geometry:{kind:'plane',plane:{origin:point(...origin),xStep:point(...xStep),yStep:point(...yStep),tileX,tileY},field:{kind:'rectangular',edgeRates:rates.map(fixed)}},stops:colors.map((srgb,i)=>({position:i,srgb:alpha&&i===0?[...srgb.slice(0,3),0]:[...srgb]})),tile:'clamp',interpolation,alpha:'straight'}}}]};
}
const linear=c=>c<=.04045?c/12.92:((c+.055)/1.055)**2.4;
const srgb=c=>c<=.0031308?12.92*c:1.055*c**(1/2.4)-.055;
const records=[];let verifiedPixels=0,maximumDifference=0;
function save(name,p,s,oracle,maximum){
 const path=out+'/'+name;
 records.push({name,request:put(path+'.json',p.json),sceneRequest:put(path+'.scene.json',s.json),response:put(path+'.response.json',p.metadata),sceneResponse:put(path+'.scene-response.json',s.metadata),frame:put(path+'.frame',Buffer.from(p.frame.buffer)),sceneFrame:put(path+'.scene.frame',Buffer.from(s.frame.buffer)),pixels:put(path+'.rgba',p.pixels),...(oracle?{reference:put(path+'.reference.rgba',oracle),verifiedPixels:10537,maximumChannelDifference:maximum}:{})});
}
for(const plane of planes)for(const [field,rates] of fields)for(const interpolation of ['srgb','linearSrgb','officeGamma1875']){
 const name=`${plane[0]}-${field}-${interpolation}`,q=request(plane,rates,interpolation),p=run(q),s=run(scene(q),true);
 assert.equal(p.response.status,'rendered',name+': '+p.metadata);assert.equal(p.frame[1],11);assert.equal(p.calls,1);assert.deepEqual(p.pixels,s.pixels);
 const [,o,a,b,tx,ty]=plane,det=a[0]*b[1]-b[0]*a[1],oracle=Buffer.alloc(p.pixels.length);let maximum=0;
 for(let y=0;y<41;y++)for(let x=0;x<257;x++){
  const px=x+.5-o[0],py=y+.5-o[1],u=tiling((px*b[1]-py*b[0])/det,tx),v=tiling((a[0]*py-a[1]*px)/det,ty);
  let t=0;for(const [i,d] of [u,v,1-u,1-v].entries())if(rates[i]>0)t=Math.max(t,1-d*rates[i]);
  const pixel=[0,0,0,255];
  for(let c=0;c<3;c++){
   const first=colors[0][c],last=colors[1][c];
   const value=interpolation==='linearSrgb'?srgb(linear(first)+(linear(last)-linear(first))*t):first+(last-first)*(interpolation==='officeGamma1875'?(last>first?1-(1-t)**1.875:t**1.875):t);
   pixel[c]=Math.round(value*255);
  }
  const at=(y*257+x)*4;oracle.set(pixel,at);
  for(let c=0;c<4;c++){const d=Math.abs(p.pixels[at+c]-pixel[c]);assert(d<=1,`${name} (${x},${y}) c${c}: ${p.pixels[at+c]} vs ${pixel[c]}`);maximum=Math.max(maximum,d);}verifiedPixels++;
 }
 maximumDifference=Math.max(maximumDifference,maximum);save(name,p,s,oracle,maximum);
}
for(const name of ['clip','snapshot','alpha']){
 const q=request(planes[0],fields[0][1],'officeGamma1875',name==='alpha');
 if(name==='clip'){q.clips=[{parent:null,path:0,origin:point(0,0)}];q.draws[0].clip=0;}
 if(name==='snapshot')q.draws.push({path:0,origin:point(0,0),brush:{kind:'snapshot',afterDraws:1},blend:'source'});
 const p=run(q),s=run(scene(q),true);assert.equal(p.response.status,'rendered');assert.deepEqual(p.pixels,s.pixels);
 if(name!=='alpha')assert.deepEqual(p.pixels,fs.readFileSync(records.find(r=>r.name==='mirror-point-officeGamma1875').pixels.path));
 save(name,p,s);
}
const negatives=[];
for(const [name,mutate] of [
 ['negative-rate',f=>f.edgeRates[0]=fixed(-1)],['disabled-uncertain',f=>{f.edgeRates[0]='0';f.uncertainty=['1','0','0','0'];}],['crosses-zero',f=>f.uncertainty=[fixed(2),'0','0','0']],['imprecise-rate',f=>f.uncertainty=[fixed(.01),'0','0','0']],['overflow-rate',f=>f.edgeRates[0]=String((1n<<127n)-1n)],['negative-uncertainty',f=>f.uncertainty=['-1','0','0','0']],['wrong-length',f=>f.edgeRates.pop()],
]){
 const q=request(planes[0],fields[0][1],'srgb');mutate(q.draws[0].brush.gradient.geometry.field);const r=run(q);assert.equal(r.response.status,'error',name);assert.equal(r.calls,0);assert.equal(r.pixels.length,0);
 negatives.push({name,request:put(out+'/'+name+'.invalid.json',r.json),response:put(out+'/'+name+'.invalid-response.json',r.metadata)});
}
const first=records[0],cliDir=fs.mkdtempSync(root+'/cli-'),target=cliDir+'/page.rgba',failure=cliDir+'/failure.rgba';
const cli=(q,p)=>spawnSync('target/debug/mo-cli',['render-paths',q,p],{env:{},timeout:60000,maxBuffer:1<<26});
const created=cli(first.request.path,target);assert.equal(created.status,0,created.stderr.toString());assert.deepEqual(fs.readFileSync(target),fs.readFileSync(first.pixels.path));assert.equal(cli(first.request.path,target).status,1);assert.deepEqual(fs.readFileSync(target),fs.readFileSync(first.pixels.path));
const bad=cli(negatives[0].request.path,failure);assert.equal(bad.status,0);assert.equal(JSON.parse(bad.stdout).status,'error');assert(!fs.existsSync(failure));
const result={format:'musteroffice.rect-gradient-parity/1',pairedCalls:paired,verifiedPixels,maximumDifference,cases:records,negatives,cli:{create:true,overwriteRefused:true,failureOutputAbsent:true,pixels:entry(target)}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({paired,verifiedPixels,maximumDifference,negative:negatives.length}));
