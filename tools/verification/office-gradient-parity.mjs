/** Real public runtimes and independent double-precision pow reference. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/office-gradient/ts-raster/index.js';
import factory from '../../.codex-work/office-gradient/component/mo-skia.mjs';
import {scene} from './compositing-fixtures.mjs';
const root='.codex-work/office-gradient',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
assert(component.supportsOfficeGradients);
const wasm=createRequire(import.meta.url)('../../.codex-work/office-gradient/wasm-node/mo_wasm.js');
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
const wrap=(t,tile)=>tile==='repeat'?t-Math.floor(t):tile==='mirror'?1-Math.abs(((t%2+2)%2)-1):Math.max(0,Math.min(1,t));
function request(kind,tile,colors,middle){
 const geometry=kind==='linear'?{kind,start:point(32,0),end:point(224,0)}:kind==='radial'?{kind,center:point(128,8),radius:fixed(80)}:
 {kind:'plane',plane:{origin:point(32,0),xStep:point(192,0),yStep:point(0,17),tileX:'mirror',tileY:'repeat'},field:{kind:'linear',coefficients:[fixed(.75),fixed(.25),fixed(-.125)]}};
 const stops=colors.map((srgb,i)=>({position:i===0?0:i===colors.length-1?1:middle,srgb:[...srgb]}));
 return {clips:[],viewport:{width:257,height:17,origin:point(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:fixed(1/256),background:[0,0,0,0]},paths:[{fillRule:'nonzero',commands:[{kind:'move',to:point(0,0)},{kind:'line',to:point(257,0)},{kind:'line',to:point(257,17)},{kind:'line',to:point(0,17)},{kind:'close'}]}],draws:[{path:0,origin:point(0,0),brush:{kind:'gradient',gradient:{geometry,stops,tile,interpolation:'officeGamma1875',alpha:'straight'}}}]};
}
const a=[25/255,204/255,102/255,1],b=[230/255,51/255,153/255,1];
const ramps=[['red-blue',[[1,0,0,1],[0,0,1,1]],null],['mixed',[a,b],null],['alpha',[[...a.slice(0,3),0],b],null],['symmetric',[a,b,a],.375],['symmetric-alpha',[[...a.slice(0,3),.25],b,[...a.slice(0,3),.25]],.125],['extended',[[-.5,.2,1.5,.5],[1.5,.2,-.5,1]],null]];
const records=[];let verifiedPixels=0,maximumDifference=0;
for(const kind of ['linear','radial','plane'])for(const tile of ['clamp','repeat','mirror','decal'])for(const [ramp,colors,mid] of ramps){
 const name=`${kind}-${tile}-${ramp}`,q=request(kind,tile,colors,mid),p=run(q),s=run(scene(q),true);
 assert.equal(p.response.status,'rendered',name+': '+p.metadata);assert.equal(p.frame[1],10);assert.equal(p.calls,1);assert.deepEqual(p.pixels,s.pixels);
 const oracle=Buffer.alloc(p.pixels.length);let maximum=0;
 for(let y=0;y<17;y++)for(let x=0;x<257;x++){
  let t=kind==='linear'?(x+.5-32)/192:kind==='radial'?Math.hypot(x+.5-128,y+.5-8)/80:.75*wrap((x+.5-32)/192,'mirror')+.25*((y+.5)/17)-.125;
  const mask=tile==='decal'&&(t<0||t>1)?0:1;t=wrap(t,tile);if(mid!==null)t=t<=mid?t/mid:(1-t)/(1-mid);
  const alpha=(colors[0][3]+(colors[1][3]-colors[0][3])*t)*mask,rgba=[0,0,0,Math.round(alpha*255)];
  for(let c=0;c<3;c++){
   const first=colors[0][c],last=colors[1][c],ratio=last>first?1-Math.pow(1-t,1.875):Math.pow(t,1.875);
   rgba[c]=Math.round(Math.max(0,Math.min(1,first+(last-first)*ratio))*alpha*255);
  }
  const at=(y*257+x)*4;oracle.set(rgba,at);
  for(let c=0;c<4;c++){const d=Math.abs(p.pixels[at+c]-rgba[c]);assert(d<=1,`${name} (${x},${y}) c${c}: ${p.pixels[at+c]} vs ${rgba[c]}`);maximum=Math.max(maximum,d);}verifiedPixels++;
 }
 maximumDifference=Math.max(maximumDifference,maximum);const path=out+'/'+name;
 records.push({name,request:put(path+'.json',p.json),sceneRequest:put(path+'.scene.json',s.json),response:put(path+'.response.json',p.metadata),sceneResponse:put(path+'.scene-response.json',s.metadata),frame:put(path+'.frame',Buffer.from(p.frame.buffer)),sceneFrame:put(path+'.scene.frame',Buffer.from(s.frame.buffer)),pixels:put(path+'.rgba',p.pixels),reference:put(path+'.reference.rgba',oracle),verifiedPixels:4369,maximumChannelDifference:maximum});
}
for(const name of ['clip','snapshot']){
 const q=request('plane','mirror',[a,b,a],.375);
 if(name==='clip'){q.clips=[{parent:null,path:0,origin:point(0,0)}];q.draws[0].clip=0;}
 else q.draws.push({path:0,origin:point(0,0),brush:{kind:'snapshot',afterDraws:1},blend:'source'});
 const p=run(q),s=run(scene(q),true);assert.equal(p.response.status,'rendered');assert.deepEqual(p.pixels,s.pixels);
 assert.deepEqual(p.pixels,fs.readFileSync(records.find(r=>r.name==='plane-mirror-symmetric').pixels.path));
 const path=out+'/'+name;records.push({name,request:put(path+'.json',p.json),sceneRequest:put(path+'.scene.json',s.json),response:put(path+'.response.json',p.metadata),sceneResponse:put(path+'.scene-response.json',s.metadata),frame:put(path+'.frame',Buffer.from(p.frame.buffer)),sceneFrame:put(path+'.scene.frame',Buffer.from(s.frame.buffer)),pixels:put(path+'.rgba',p.pixels)});
}
const negatives=[];
for(const [name,mutate] of [
 ['premultiplied',g=>g.alpha='premultiplied'],['shifted-end',g=>g.stops.at(-1).position=.99],['near-end',g=>g.stops.at(-1).position=.9999999999999999],['asymmetric',g=>g.stops.at(-1).srgb[0]=.2],['near-color',g=>g.stops.at(-1).srgb[0]+=.0000000000000001],['unequal-alpha',g=>g.stops.at(-1).srgb[3]=.5],['duplicate-middle',g=>g.stops[1].position=0],['four-stops',g=>g.stops.splice(1,0,g.stops[0])],['nan-color',g=>g.stops[1].srgb[0]=null],
]){
 const q=request('linear','clamp',[a,b,a],.375);mutate(q.draws[0].brush.gradient);const r=run(q);assert.equal(r.response.status,'error',name);assert.equal(r.calls,0);assert.equal(r.pixels.length,0);
 negatives.push({name,request:put(out+'/'+name+'.invalid.json',r.json),response:put(out+'/'+name+'.invalid-response.json',r.metadata)});
}
const first=records[0],cliDir=fs.mkdtempSync(root+'/cli-'),target=cliDir+'/page.rgba';
const cli=()=>spawnSync('target/debug/mo-cli',['render-paths',first.request.path,target],{env:{},timeout:60000,maxBuffer:1<<26});
const created=cli();assert.equal(created.status,0,created.stderr.toString());assert.deepEqual(fs.readFileSync(target),fs.readFileSync(first.pixels.path));assert.equal(cli().status,1);assert.deepEqual(fs.readFileSync(target),fs.readFileSync(first.pixels.path));
const result={format:'musteroffice.office-gradient-parity/1',pairedCalls:paired,verifiedPixels,maximumDifference,cases:records,negatives,cli:{create:true,overwriteRefused:true,pixels:entry(target)}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({paired,verifiedPixels,maximumDifference,negative:negatives.length}));
