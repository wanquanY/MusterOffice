/** Public field lowering; all source parameters are explicit Q32 values. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import factory from '../../.codex-work/elliptic-render/component/mo-skia.mjs';
import {scene} from './compositing-fixtures.mjs';
const root='.codex-work/elliptic-source',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/elliptic-source/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/elliptic-render/component/mo-skia.wasm')));
assert(component.supportsEllipticGradients);
const fixed=n=>String(BigInt(Math.round(n*4294967296))),point=(x,y)=>({x:fixed(x),y:fixed(y)});
function request(o={}){
 const {width=128,height=64,matrix=[128,0,0,0,64,0],scale=[.8,.6],field=[0,0,0,0],tile=[0,0],draws=1,office=false}=o;
 const m=matrix.map(Math.fround),v=[...scale,...field].map(Math.fround),modes=['clamp','repeat','mirror'];
 const brush={kind:'gradient',gradient:{geometry:{kind:'plane',plane:{origin:point(m[2],m[5]),xStep:point(m[0],m[3]),yStep:point(m[1],m[4]),tileX:modes[tile[0]],tileY:modes[tile[1]]},field:{kind:'elliptic',tileScale:v.slice(0,2).map(fixed),innerCenter:v.slice(2,4).map(fixed),innerRadii:v.slice(4,6).map(fixed)}},stops:(office?[[0,32],[1,224]]:[[0,32],[.5,128],[1,224]]).map(([position,g])=>({position,srgb:[g/255,g/255,g/255,1]})),tile:'clamp',interpolation:office?'officeGamma1875':'srgb',alpha:'straight'}};
 return {clips:[],viewport:{width,height,origin:point(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:fixed(1/256),background:[255,255,255,255]},paths:[{fillRule:'nonzero',commands:[{kind:'move',to:point(0,0)},{kind:'line',to:point(width,0)},{kind:'line',to:point(width,height)},{kind:'line',to:point(0,height)},{kind:'close'}]}],draws:Array.from({length:draws},()=>({path:0,origin:point(0,0),brush}))};
}
let frame,calls=0;
const backend={raster(f){frame=f.slice();calls++;return component.raster(f);},invalidate(){component.invalidate();}};
function run(name,q,isScene=false,previous){
 const json=typeof q==='string'?q:JSON.stringify(q),b=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(b.length);
 const n=spawnSync('target/debug/mo-raster-worker',isScene?['--scene']:[],{input:Buffer.concat([h,b]),env:{},timeout:60000,maxBuffer:1<<26});
 assert.equal(n.status,0,n.stderr.toString());const ml=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+ml).toString(),pixels=n.stdout.subarray(8+ml);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 frame=null;calls=0;const w=isScene?wasm.render_scene(json,backend):wasm.render_paths(json,backend);
 assert.equal(w.metadata,metadata,name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,name);assert(!component.invalid,name);
 if(previous){assert.equal(metadata,load(previous.response).toString(),name);if(previous.pixels)assert.deepEqual(pixels,load(previous.pixels));if(previous.frame)assert.deepEqual(Buffer.from(frame.buffer),load(previous.frame));}
 const prefix=out+'/'+name,record={name,request:put(prefix+'.json',json),response:put(prefix+'.response.json',metadata),calls,status:JSON.parse(metadata).status};
 if(frame)record.frame=put(prefix+'.frame',Buffer.from(frame.buffer));
 if(pixels.length)record.pixels=put(prefix+'.rgba',pixels);
 return {record,pixels,response:JSON.parse(metadata)};
}
const records=[],old=JSON.parse(fs.readFileSync('.codex-work/elliptic-render/components.json'));
for(const c of old.cases){
 if(c.name==='precision-failure')continue; // Subnormal input is below public Q32 resolution.
 const q=request(c.options),r=run('new-'+c.name,q),s=run('scene-'+c.name,scene(q),true);
 assert.equal(r.record.status,c.expectedStatus===0?'rendered':'error',c.name+': '+JSON.stringify(r.response));
 assert.equal(r.record.calls,1);assert.deepEqual(r.pixels,s.pixels);
 if(c.expectedStatus===0){assert.deepEqual(r.pixels,load(c.pixels),c.name);assert(r.response.info.work.ellipticGradients);}
 else{assert.equal(r.pixels.length,0);assert.equal(r.response.error.code,'COMPONENT_FAILURE');}
 records.push(r.record,s.record);
}
const field=q=>q.draws[0].brush.gradient.geometry.field;
for(const [name,mutate] of [
 ['scale-zero',q=>field(q).tileScale[0]='0'],['scale-over',q=>field(q).tileScale[0]=fixed(1.1)],['radius-negative',q=>field(q).innerRadii[1]='-1'],
 ['center-range',q=>field(q).innerCenter[0]=fixed(32769)],['uncertain-zero',q=>field(q).uncertainty=[fixed(1),'0','0','0','0','0']],
 ['negative-error',q=>field(q).uncertainty=['0','0','-1','0','0','0']],['geometry-budget',q=>field(q).uncertainty=Array(6).fill(fixed(.01))],
 ['wrong-arity',q=>field(q).tileScale.pop()],['unknown-field',q=>field(q).script='unused']
]){
 const q=request();mutate(q);const r=run('invalid-'+name,q);assert.equal(r.record.status,'error',name);assert.equal(r.record.calls,0);assert.equal(r.pixels.length,0);records.push(r.record);
}
for(let i=0;i<8;i++){
 const q=request({width:8,height:8,matrix:[67/3,5/7,-32/7,3/13,41/7,7/3],scale:[1/3,1/2],field:[-1/5,3/20,2/5,7/10]});
 const f=field(q);for(const name of ['tileScale','innerCenter','innerRadii'])f[name]=f[name].map((v,j)=>String(BigInt(v)+BigInt(3+i+j)));
 f.uncertainty=Array.from({length:6},(_,j)=>String((i+1)*(j+1)));
 const p=q.draws[0].brush.gradient.geometry.plane;
 p.uncertainty={origin:{x:'13',y:'17'},xStep:{x:'19',y:'23'},yStep:{x:'29',y:'31'}};
 p.origin.x=String(BigInt(p.origin.x)+7n);p.yStep.y=String(BigInt(p.yStep.y)-11n);
 const r=run('parameter-'+i,q);assert.equal(r.record.status,'rendered',JSON.stringify(r.response));records.push(r.record);
}
const recovery=run('recovery',request());assert.equal(recovery.record.status,'rendered');records.push(recovery.record);
const prior=JSON.parse(fs.readFileSync('.codex-work/rect-gradient/parity.json'));
for(const c of prior.cases){
 records.push(run('prior-'+c.name,load(c.request).toString(),false,c).record);
 records.push(run('prior-scene-'+c.name,load(c.sceneRequest).toString(),true,{response:c.sceneResponse,pixels:c.pixels,frame:c.sceneFrame}).record);
}
for(const c of prior.negatives)records.push(run('prior-invalid-'+c.name,load(c.request).toString(),false,c).record);
const report={format:'musteroffice.elliptic-source-runtime/1',pairedCalls:records.length,priorUnchanged:prior.cases.length*2+prior.negatives.length,cases:records,lowLevelEquivalent:20,lateFailures:2,recovery:true,excluded:[{name:'precision-failure',reason:'The prior raw shader case uses 2^-149, below the public Q32 parameter resolution; public scale-zero is rejected before backend.'}]};
fs.writeFileSync(root+'/runtime.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({paired:records.length,prior:report.priorUnchanged,newEquivalent:20}));
