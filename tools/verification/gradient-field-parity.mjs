/** Public evaluated-plane pixels, Native/WASM parity, malformed requests. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/gradient-field/ts-raster/index.js';
import factory from '../../.codex-work/gradient-field/component/mo-skia.mjs';
import {scene} from './compositing-fixtures.mjs';
const root='.codex-work/gradient-field',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
assert(component.supportsGradientPlanes);
const wasm=createRequire(import.meta.url)('../../.codex-work/gradient-field/wasm-node/mo_wasm.js');
const fixed=n=>String(BigInt(Math.round(n*4294967296))),point=(x,y)=>({x:fixed(x),y:fixed(y)});
let frame,calls=0,paired=0;
const backend={raster(f){frame=f.slice();calls++;return component.raster(f);},invalidate(){component.invalidate();}};
function run(q,s=false){
 const json=JSON.stringify(q),b=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(b.length);
 const n=spawnSync('target/debug/mo-raster-worker',s?['--scene']:[],{input:Buffer.concat([h,b]),env:{},timeout:60000,maxBuffer:1<<26});
 assert.equal(n.status,0,n.stderr.toString());const m=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+m).toString(),pixels=n.stdout.subarray(8+m);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 calls=0;frame=null;const w=s?wasm.render_scene(json,backend):wasm.render_paths(json,backend);
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!component.invalid);paired++;
 return {json,metadata,pixels,response:JSON.parse(metadata),frame,calls};
}
function request(a,tileX,tileY){
 const [a00,a01,a10,a11]=a;
 return {clips:[],viewport:{width:32,height:24,origin:point(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:fixed(1/256),background:[0,0,0,0]},paths:[{fillRule:'nonzero',commands:[{kind:'move',to:point(0,0)},{kind:'line',to:point(32,0)},{kind:'line',to:point(32,24)},{kind:'line',to:point(0,24)},{kind:'close'}]}],draws:[{path:0,origin:point(0,0),brush:{kind:'gradient',gradient:{geometry:{kind:'plane',plane:{origin:point(4,2),xStep:point(a00,a10),yStep:point(a01,a11),tileX,tileY},field:{kind:'linear',coefficients:[fixed(.5),fixed(.5),fixed(0)]}},stops:[{position:0,srgb:[1,0,0,1]},{position:1,srgb:[0,0,1,1]}],tile:'clamp',interpolation:'srgb',alpha:'straight'}}}]};
}
const wrap=(v,mode)=>mode==='clamp'?Math.max(0,Math.min(1,v)):mode==='repeat'?v-Math.floor(v):1-Math.abs(((v%2+2)%2)-1);
const records=[];let checked=0,maxDifference=0;
for(const [kind,a] of [['axis',[8,0,0,8]],['shear',[8,4,0,8]],['reflected',[-8,4,4,8]]])for(const tx of ['clamp','repeat','mirror'])for(const ty of ['clamp','repeat','mirror']){
 const q=request(a,tx,ty),name=`${kind}-${tx}-${ty}`,p=run(q),s=run(scene(q),true);
 assert.equal(p.response.status,'rendered',name+': '+p.metadata);assert.equal(p.frame[1],9);assert.equal(p.calls,1);assert.deepEqual(p.pixels,s.pixels);
 const det=a[0]*a[3]-a[1]*a[2];
 for(let y=0;y<24;y++)for(let x=0;x<32;x++){
  const dx=x+.5-4,dy=y+.5-2,u=(a[3]*dx-a[1]*dy)/det,v=(a[0]*dy-a[2]*dx)/det,t=(wrap(u,tx)+wrap(v,ty))/2,expected=[Math.round(255*(1-t)),0,Math.round(255*t),255],i=(y*32+x)*4;
  for(let c=0;c<4;c++){const delta=Math.abs(p.pixels[i+c]-expected[c]);assert(delta<=1,`${name} (${x},${y}) channel ${c}: ${p.pixels[i+c]} != ${expected[c]}`);maxDifference=Math.max(delta,maxDifference);}checked++;
 }
 const path=out+'/'+name;records.push({name,request:put(path+'.json',p.json),sceneRequest:put(path+'.scene.json',s.json),response:put(path+'.response.json',p.metadata),sceneResponse:put(path+'.scene-response.json',s.metadata),frame:put(path+'.frame',Buffer.from(p.frame.buffer)),sceneFrame:put(path+'.scene.frame',Buffer.from(s.frame.buffer)),pixels:put(path+'.rgba',p.pixels)});
}
// Alpha/interpolation/clip/composition share the same native color and draw stages.
for(const [name,mutate] of [
 ['alpha-straight',q=>q.draws[0].brush.gradient.stops[0].srgb[3]=0],
 ['alpha-premul',q=>{q.draws[0].brush.gradient.stops[0].srgb[3]=0;q.draws[0].brush.gradient.alpha='premultiplied';}],
 ['linear-srgb',q=>q.draws[0].brush.gradient.interpolation='linearSrgb'],
 ['hard-stop',q=>q.draws[0].brush.gradient.stops.splice(1,0,{position:.4,srgb:[1,0,0,1]},{position:.4,srgb:[0,0,1,1]})],
 ['clip',q=>{q.clips=[{parent:null,path:0,origin:point(2,3)}];q.draws[0].clip=0;}],
 ['snapshot',q=>q.draws.push({path:0,origin:point(0,0),brush:{kind:'snapshot',afterDraws:1},blend:'source'})],
]){const q=request([8,4,0,8],'mirror','mirror');mutate(q);const p=run(q);assert.equal(p.response.status,'rendered',p.metadata);const path=out+'/'+name;records.push({name,request:put(path+'.json',p.json),response:put(path+'.response.json',p.metadata),frame:put(path+'.frame',Buffer.from(p.frame.buffer)),pixels:put(path+'.rgba',p.pixels)});}
const negatives=[];
for(const [name,mutate] of [
 ['singular',g=>g.geometry.plane.yStep=g.geometry.plane.xStep],
 ['negative-error',g=>g.geometry.field.uncertainty=['-1','0','0']],
 ['excess-error',g=>g.geometry.field.uncertainty=[fixed(.01),'0','0']],
 ['unknown-field',g=>g.geometry.field.script='ignored?'],
 ['invalid-tile',g=>g.geometry.plane.tileX='decal'],
 ['collapsed-stops',g=>g.stops.splice(1,0,{position:.5,srgb:[1,0,0,1]},{position:.500000000000001,srgb:[0,0,1,1]})],
]){const q=request([8,0,0,8],'mirror','mirror');mutate(q.draws[0].brush.gradient);const r=run(q);assert.equal(r.response.status,'error',name);assert.equal(r.calls,0);assert.equal(r.pixels.length,0);negatives.push({name,request:put(out+'/'+name+'.invalid.json',r.json),response:put(out+'/'+name+'.invalid-response.json',r.metadata)});}
const cliDir=fs.mkdtempSync(root+'/cli-'),c=records[0],output=cliDir+'/page.rgba';
const cli=()=>spawnSync('target/debug/mo-cli',['render-paths',c.request.path,output],{env:{},timeout:60000,maxBuffer:1<<26});
const r=cli();assert.equal(r.status,0,r.stderr.toString());assert.deepEqual(fs.readFileSync(output),fs.readFileSync(c.pixels.path));assert.notEqual(cli().status,0);assert.deepEqual(fs.readFileSync(output),fs.readFileSync(c.pixels.path));
const result={format:'musteroffice.gradient-field-parity/1',pairedCalls:paired,oraclePixels:checked,maxChannelDifference:maxDifference,cases:records,negatives,cli:{create:true,overwriteRefused:true,output:entry(output)}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({paired,checked,maxDifference,negative:negatives.length}));
