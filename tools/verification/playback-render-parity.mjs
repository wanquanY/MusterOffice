/** Real sampled pages through Native/WASM and the existing shared raster component. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
const root='.codex-work/playback-render',out=root+'/frames';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/playback-render/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const put=(p,b)=>{fs.writeFileSync(p,typeof b==='string'||Buffer.isBuffer(b)?b:JSON.stringify(b));return entry(p);};
let captured=null,calls=0;const backend={raster(f){calls++;captured=f.slice();return component.raster(f);},invalidate(){component.invalidate();}};
const invoke=(args,input)=>{const r=spawnSync('target/debug/mo-cli',args,{input,env:{},timeout:60000,maxBuffer:64<<20});assert.equal(r.status,0,r.stderr.toString());assert.equal(r.stderr.length,0);return r.stdout.toString().trimEnd();};
function nativeRender(json,mode='--playback-page'){
 const header=Buffer.alloc(4);header.writeUInt32LE(Buffer.byteLength(json));const r=spawnSync('target/debug/mo-raster-worker',[mode],{input:Buffer.concat([header,Buffer.from(json)]),env:{},timeout:60000,maxBuffer:64<<20});assert.equal(r.status,0,r.stderr.toString());assert.equal(r.stderr.length,0);
 const ml=r.stdout.readUInt32LE(),pl=r.stdout.readUInt32LE(4);assert.equal(r.stdout.length,8+ml+pl);return {metadata:r.stdout.subarray(8,8+ml).toString(),pixels:r.stdout.subarray(8+ml)};
}
const page=JSON.parse(fs.readFileSync('fixtures/presentations/playback/page.json')),records=[];
function request(page,at,events=null){
 const initialize=JSON.stringify({operation:'initialize',document:page.page.document});const n=invoke([],initialize);assert.equal(n,wasm.dispatch_json(initialize));const snapshot=JSON.parse(n).snapshot;assert(snapshot);
 const binding={session:'render',revision:snapshot.revision,generation:'3'};
 return {playback:{snapshot,slide:page.page.slide,binding,at,history:events===null?null:{binding:{...binding},through:{ticks:'10',timescale:1},events}},viewport:structuredClone(page.viewport),defaults:structuredClone(page.defaults)};
}
const time=(n,d=1)=>({ticks:String(n),timescale:d});
function pair(name,q,expected='rendered'){
 const p=out+'/'+records.length,json=JSON.stringify(q),req=put(p+'.request.json',json);
 const compile=invoke(['compile-playback-page',req.path]);assert.equal(compile,wasm.compile_playback_page(json),name+' compile');
 const n=nativeRender(json);calls=0;captured=null;const w=wasm.render_playback_page(json,backend);assert.equal(w.metadata,n.metadata,name+' metadata');assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels,name+' pixels');assert(!component.invalid);
 const metadata=JSON.parse(n.metadata),compiled=JSON.parse(compile);assert.equal(metadata.status,expected,name);
 if(expected==='rendered'){
  assert.equal(calls,1);assert.equal(compiled.status,'compiled');assert.deepEqual(compiled.frame.frame,metadata.info.frame);assert.equal(sha(n.pixels),metadata.info.page.scene.raster.sha256);assert.equal(metadata.info.frame.state.binding.revision,q.playback.snapshot.revision);
 }else{assert.equal(calls,0);assert.equal(compiled.status,'error');assert.deepEqual(metadata.error,compiled.error);assert.equal(n.pixels.length,0);}
 const record={name,request:req,compiled:put(p+'.compiled.json',compile),response:put(p+'.response.json',n.metadata),pixels:put(p+'.rgba',n.pixels),calls,...(captured?{frame:put(p+'.frame',Buffer.from(captured.buffer))}:{})};
 if(expected==='rendered'&&Object.values(metadata.info.frame.state.rotations).every(r=>r.denominator==='1')){
  const staticQ={page:{document:structuredClone(q.playback.snapshot.document),slide:q.playback.slide},viewport:q.viewport,defaults:q.defaults};
  for(const [id,r]of Object.entries(metadata.info.frame.state.rotations))staticQ.page.document.objects[id].transform.rotation=Number(r.numerator);
  const result=nativeRender(JSON.stringify(staticQ),'--page');assert.deepEqual(result.pixels,n.pixels,name+' integer static pixels');record.integerStaticReference=put(p+'.static.json',staticQ);
 }
 records.push(record);return record;
}
for(const [i,at]of [time(0),time(1,3),time(2,3),time(999,1000),time(1),time(1001,1000),time(4,3),time(5,3),time(2),time(1,3)].entries())pair('grouped-'+i,request(page,at));
const nested=structuredClone(page),nd=nested.page.document;
const inner=structuredClone(nd.objects['group:1']);inner.id='group:inner';inner.parent={kind:'group',id:'group:1'};inner.transform.origin={x:'150000',y:'80000'};inner.transform.size={width:'600000',height:'380000'};inner.transform.flipHorizontal=false;inner.transform.flipVertical=true;nd.objects[inner.id]=inner;
nd.objects['group:1'].content.children=[inner.id];for(const id of ['shape:1','shape:2'])nd.objects[id].parent.id=inner.id;
nd.timelines['slide:1'].nodes.push({id:'rotation:inner',start:{kind:'at',offset:time(0)},duration:time(7,3),repeatMilli:1500,fill:'freeze',effect:{kind:'rotation',target:inner.id,from:2100000,to:4100000}});
for(const [i,at]of [time(1,7),time(2,7),time(1001,30000),time(11,13),time(17,19),time(23,29),time(31,37),time(41,43),time(3,2),time(2),time(3),time(4)].entries())pair('nested-'+i,request(nested,at));
const rigid=structuredClone(page),d=rigid.page.document,o=d.objects['shape:1'];d.objects={'shape:1':o};d.slides['slide:1'].objects=['shape:1'];o.parent={kind:'slide',id:'slide:1'};o.transform.origin={x:'1100000',y:'900000'};o.transform.size={width:'1000000',height:'600000'};o.transform.rotation=0;
d.timelines['slide:1'].nodes=[{id:'rotate',start:{kind:'at',offset:time(0)},duration:time(1),repeatMilli:1000,fill:'freeze',effect:{kind:'rotation',target:'shape:1',from:0,to:21600000}}];
for(let i=0;i<=4;i++)pair('rigid-'+i,request(rigid,time(i,4)));
const interactive=structuredClone(page);interactive.page.document.timelines['slide:1'].nodes[0].start={kind:'click',target:null,delay:time(0)};
const events=[{generation:'3',sequence:1,at:time(1,2),event:{kind:'click',target:null}}];
for(const [i,at]of [time(0),time(1,3),time(1,2),time(1),time(3),time(1,3)].entries())pair('interactive-'+i,request(interactive,at,events));
pair('missing-history',request(interactive,time(1)), 'error');
const stale=request(page,time(1));stale.playback.binding.revision='0'.repeat(64);pair('stale-snapshot',stale,'error');
const unsupported=request(page,time(1));unsupported.playback.snapshot.document.objects['shape:1'].appearance.fill={kind:'inherit'};const init=JSON.parse(invoke([],JSON.stringify({operation:'initialize',document:unsupported.playback.snapshot.document})));unsupported.playback.snapshot=init.snapshot;unsupported.playback.binding.revision=init.snapshot.revision;pair('unsupported-paint',unsupported,'error');
const failed=request(page,time(1,3));failed.viewport.coordinateTolerance='1';pair('precision-budget',failed,'error');
// A failed frame must not poison the component or later independent requests.
pair('recovery',request(page,time(1,3)));
const positive=records.find(c=>c.name==='grouped-1'),negative=records.find(c=>c.name==='missing-history');const dir=fs.mkdtempSync(root+'/cli-'),path=dir+'/frame.rgba';
const cli=invoke(['render-playback-page',positive.request.path,path]);assert.deepEqual(fs.readFileSync(path),fs.readFileSync(positive.pixels.path));
const before=entry(path),again=spawnSync('target/debug/mo-cli',['render-playback-page',positive.request.path,path],{env:{},timeout:60000});assert.notEqual(again.status,0);assert.deepEqual(entry(path),before);
const absent=dir+'/failure.rgba',failure=invoke(['render-playback-page',negative.request.path,absent]);assert(!fs.existsSync(absent));assert.equal(JSON.parse(failure).status,'error');
const result={format:'musteroffice.playback-render-parity/1',cases:records,pairedCalls:records.length*2,rendered:records.filter(c=>c.calls===1).length,cli:{created:before,response:put(dir+'/created.json',cli),overwriteExit:again.status,failure:put(dir+'/failure.json',failure),failureOutputAbsent:true},artifacts:[entry('target/debug/mo-cli'),entry('target/debug/mo-raster-worker'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({pairedCalls:result.pairedCalls,rendered:result.rendered}));
