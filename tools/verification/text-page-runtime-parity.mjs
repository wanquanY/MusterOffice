import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import skiaFactory from '../../.codex-work/skia/mo-skia.mjs';

const root='.codex-work/text-page-runtime';
const wasm=createRequire(import.meta.url)(`../../${root}/wasm-node/mo_wasm.js`);
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const fixtures=JSON.parse(fs.readFileSync(root+'/fixtures.json'));
const fontPath='fixtures/fonts/owned.ttf', font=fs.readFileSync(fontPath);
const fontManifest=JSON.parse(fs.readFileSync('fixtures/fonts/manifest-paragraph.json')).manifest;
const hbModule=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const skiaModule=new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm'));
const shaper=await ShapingComponent.create(hbFactory,hbModule);
const raster=await RasterComponent.create(skiaFactory,skiaModule);
let textCalls=0,rasterCalls=0;
const text={...Object.fromEntries(['shapeBatch','measureBatch','outlineBatch'].map(name=>[name,(...args)=>{textCalls++;return shaper[name](...args);}])) ,invalidate(){shaper.invalidate();}};
const paint={raster(frame){rasterCalls++;return raster.raster(frame);},invalidate(){raster.invalidate();}};
const records=[];
function request(source){return {profile:'drawingml-solid-text-page-q32-draft-v1',page:{expectedSourceSha256:sha(source),slide:'/ppt/slides/slide1.xml',profile:'drawingml-static-solid-page-v1-draft',colorContext:{systemColors:{},placeholder:null},viewport:{width:400,height:300,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:4000},coordinateTolerance:'16777216',background:[255,255,255,255]}},fonts:structuredClone(fontManifest)};}
function frame(json,source,fonts){const q=Buffer.from(json),h=Buffer.alloc(12);h.writeUInt32LE(q.length);h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);return Buffer.concat([h,q,source,fonts]);}
function decode(bytes){const m=bytes.readUInt32LE(),n=bytes.readUInt32LE(4);assert.equal(bytes.length,8+m+n);return {metadata:bytes.subarray(8,8+m).toString(),pixels:bytes.subarray(8+m)};}
function native(json,source,fonts){const n=spawnSync('target/debug/mo-raster-worker',['--pptx-text-page'],{input:frame(json,source,fonts),timeout:60000,maxBuffer:80*1024*1024,env:{}});assert.equal(n.status,0,n.stderr?.toString());return decode(n.stdout);}
function run(name,source,q,fonts=font,expected={status:'rendered'},validRequest=true){
 const json=typeof q==='string'?q:JSON.stringify(q),n=native(json,source,fonts);
 textCalls=0;rasterCalls=0;
 const w=wasm.render_pptx_text_page(json,source,fonts,text,paint);
 assert.equal(w.metadata,n.metadata,name);assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels,name);
 const result=JSON.parse(n.metadata);assert.equal(result.status,expected.status,name+': '+n.metadata);
 if(result.status==='error'){
  assert.equal(n.pixels.length,0);assert.equal(rasterCalls,0);assert.equal(result.error.stage,expected.stage,name);
  if(expected.code)assert.equal(result.error.error.code,expected.code,name);
  if(expected.detail){assert.equal(result.error.detail.kind,expected.detail,name);assert.equal(result.error.error.location.part,'/ppt/slides/slide1.xml');assert.ok([42,43].includes(result.error.error.location.object));}
  if(expected.textCalls!==undefined)assert.equal(textCalls,expected.textCalls,name);
 }else{
  assert.equal(result.info.profile,q.profile);assert.equal(result.info.textWork.componentCalls,textCalls);assert.equal(rasterCalls,1);
  const i=result.info.page.scene.raster;assert.equal(i.sha256,sha(n.pixels));assert.equal(i.byteLength,String(n.pixels.length));assert.equal(n.pixels.length,400*300*4);
 }
 const prefix=`${root}/${name}`;
 for(const [suffix,data] of [['request.json',json],['response.json',n.metadata],['source.pptx',source],['fonts.bin',fonts]])fs.writeFileSync(prefix+'.'+suffix,data);
 const rec={name,validRequest,request:entry(prefix+'.request.json'),response:entry(prefix+'.response.json'),source:entry(prefix+'.source.pptx'),fonts:entry(prefix+'.fonts.bin'),textCalls,rasterCalls};
 if(n.pixels.length){fs.writeFileSync(prefix+'.rgba',n.pixels);rec.pixels=entry(prefix+'.rgba');}
 records.push(rec);return {result,...n,rec};
}
for(const [i,c]of fixtures.positive.entries()){
 const source=load(c.source),r=run('native-source-'+i,source,request(source));
 assert.deepEqual(r.pixels,load(c.pixels));const plan=JSON.parse(load(c.result));
 assert.deepEqual(r.result.info.textWork,plan.textWork);assert.equal(r.result.info.textFrames,plan.texts.length);
 r.rec.priorPixelOracle=c.pixels;
}
for(const c of fixtures.negative){
 const source=load(c.source);run(c.name,source,request(source),font,{status:'error',stage:'page',code:'MAPPING_NOT_IMPLEMENTED',detail:c.detail,textCalls:c.detail==='glyphPaintConflict'?3:0});
}
const source=load(fixtures.positive[0].source);
for(const [name,change,stage,code,valid=true] of [
 ['unknown-profile',q=>q.profile='html','request','INPUT_INVALID',false],
 ['unknown-member',q=>q.script='code','request','INPUT_INVALID',false],
 ['digest-conflict',q=>q.page.expectedSourceSha256='0'.repeat(64),'source','SOURCE_CONFLICT'],
 ['unverified-name',q=>q.fonts.faces[0].family.expected='Unverified','fonts','INPUT_INVALID'],
 ['missing-slide',q=>q.page.slide='/ppt/missing.xml','page','INPUT_INVALID'],
 ['font-range',q=>q.fonts.fonts[0].byteLength='1921','fonts','INPUT_INVALID'],
 ['zero-scale',q=>q.page.viewport.scale.numerator=0,'page','INPUT_INVALID'],
 ['numeric-resource-offset',q=>q.fonts.fonts[0].offset=0,'request','INPUT_INVALID',false],
]){const q=request(source);change(q);run(name,source,q,font,{status:'error',stage,code,textCalls:0},valid);}
const json=JSON.stringify(request(source));
run('duplicate-field',source,json.replace('"fonts":','"fonts":{},"fonts":'),font,{status:'error',stage:'request',code:'INPUT_INVALID',textCalls:0},false);
const badFont=Buffer.from(font);badFont[0]^=1;
run('changed-font-bytes',source,request(source),badFont,{status:'error',stage:'fonts',code:'RESOURCE_CONFLICT',textCalls:0});
run('empty-font-bundle',source,request(source),Buffer.alloc(0),{status:'error',stage:'fonts',code:'INPUT_INVALID',textCalls:0});

// The existing native leaf edit produces a real new PPTX, then the same page
// API renders that revision. Reusing the old source binding must fail.
const edit={expectedSourceSha256:sha(source),edits:[{target:{part:'/ppt/slides/slide1.xml',objectId:42,paragraph:0,run:0},expectedText:'A',replacement:'AAA'}]};
const editPath=root+'/edit.json',editedPath=root+'/edited.pptx';fs.writeFileSync(editPath,JSON.stringify(edit));
if(fs.existsSync(editedPath))fs.unlinkSync(editedPath);
const editCli=spawnSync('target/debug/mo-cli',['pptx-edit-text',editPath,fixtures.positive[0].source.path,editedPath],{timeout:60000,encoding:'utf8'});assert.equal(editCli.status,0,editCli.stderr);
const edited=fs.readFileSync(editedPath);assert.deepEqual(edited,Buffer.from(wasm.edit_pptx_text(JSON.stringify(edit),source)));
assert.deepEqual(load(fixtures.positive[0].source),source);assert.notEqual(sha(edited),sha(source));
const editedRender=run('edited-native-text',edited,request(edited));assert.notEqual(sha(editedRender.pixels),records[0].pixels.sha256);
run('edited-stale-binding',edited,request(source),font,{status:'error',stage:'source',code:'SOURCE_CONFLICT',textCalls:0});
const editRoundtrip={request:entry(editPath),input:fixtures.positive[0].source,output:entry(editedPath),nativeWasmPackagesIdentical:true,changedPixels:true,staleBindingRejected:true};

// CLI publication validates metadata/digest and cannot overwrite a prior file.
const cliChecks=[];
for(const c of records){
 const out=`${root}/cli-${c.name}.rgba`;if(fs.existsSync(out))fs.unlinkSync(out);
 const args=['render-pptx-text-page',c.request.path,c.source.path,c.fonts.path,out];
 const result=spawnSync('target/debug/mo-cli',args,{timeout:60000,maxBuffer:8*1024*1024,encoding:'utf8'});
 assert.equal(result.status,0,result.stderr);assert.equal(result.stdout.trimEnd(),load(c.response).toString());
 assert.equal(fs.existsSync(out),!!c.pixels);
 if(c.pixels){assert.deepEqual(fs.readFileSync(out),load(c.pixels));const again=spawnSync('target/debug/mo-cli',args,{timeout:60000});assert.notEqual(again.status,0);assert.deepEqual(fs.readFileSync(out),load(c.pixels));cliChecks.push({name:c.name,pixels:entry(out),overwriteRejected:true});}
 else cliChecks.push({name:c.name,noArtifact:true});
}
// Two requests in one process keep resource and response boundaries intact.
const batchInputs=records.slice(0,2).map(c=>frame(load(c.request).toString(),load(c.source),load(c.fonts)));
const batch=spawnSync('target/debug/mo-raster-worker',['--pptx-text-page'],{input:Buffer.concat(batchInputs),timeout:60000,maxBuffer:8*1024*1024,env:{}});assert.equal(batch.status,0);
let at=0;for(const c of records.slice(0,2)){const length=8+batch.stdout.readUInt32LE(at)+batch.stdout.readUInt32LE(at+4);const r=decode(batch.stdout.subarray(at,at+length));assert.equal(r.metadata,load(c.response).toString());assert.deepEqual(r.pixels,load(c.pixels));at+=length;}assert.equal(at,batch.stdout.length);
const malformed=[];
for(const [name,counts,tail]of [['request-limit',[32*1024*1024+1,0,0],[]],['source-limit',[0,128*1024*1024+1,0],[]],['font-limit',[0,0,128*1024*1024+1],[]],['truncated-font',[0,0,4],[1]],['truncated-source',[0,4,0],[1]]]){
 const h=Buffer.alloc(12);counts.forEach((v,i)=>h.writeUInt32LE(v,4*i));const n=spawnSync('target/debug/mo-raster-worker',['--pptx-text-page'],{input:Buffer.concat([h,Buffer.from(tail)]),timeout:10000,env:{}});assert.notEqual(n.status,0);assert.equal(n.stdout.length,0);malformed.push({name,status:n.status,stdoutBytes:0});
}

// A real shaping allocation failure after layout must prevent raster and poison
// only that component instance. A fresh healthy instance still renders normally.
let raw;const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default;
const faultBytes=fs.readFileSync('.codex-work/harfbuzz/mo-hb.wasm');
const fault=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},new WebAssembly.Module(faultBytes));
let outlines=0;
const failingText={shapeBatch:(...a)=>fault.shapeBatch(...a),measureBatch:(...a)=>fault.measureBatch(...a),outlineBatch(...a){outlines++;raw._mo_hb_fail_after(0);return fault.outlineBatch(...a);},invalidate(){fault.invalidate();}};
const take=r=>{const result=JSON.parse(r.metadata);assert.equal(r.take_pixels().length,0);return result;};
rasterCalls=0;const failed=take(wasm.render_pptx_text_page(json,source,font,failingText,paint));
assert.equal(outlines,1);assert.equal(rasterCalls,0);assert.equal(failed.error.error.code,'COMPONENT_FAILURE');assert.equal(fault.invalid,true);
const reused=take(wasm.render_pptx_text_page(json,source,font,failingText,paint));assert.equal(reused.error.error.code,'HOST_FAILURE');assert.equal(rasterCalls,0);
let rasterInvalid=false;textCalls=0;
const badRaster={raster(){throw new Error('test host failure');},invalidate(){rasterInvalid=true;}};
const rasterFailure=take(wasm.render_pptx_text_page(json,source,font,text,badRaster));assert.equal(rasterFailure.error.error.code,'HOST_FAILURE');assert.equal(textCalls,6);assert.equal(rasterInvalid,true);
const healthy=wasm.render_pptx_text_page(json,source,font,text,paint);assert.equal(healthy.metadata,load(records[0].response).toString());assert.deepEqual(Buffer.from(healthy.take_pixels()),load(records[0].pixels));
const artifacts=['target/debug/mo-cli','target/debug/mo-raster-worker',`${root}/wasm-node/mo_wasm.js`,`${root}/wasm-node/mo_wasm_bg.wasm`,'.codex-work/harfbuzz/release/mo-hb.mjs','.codex-work/harfbuzz/release/mo-hb.wasm','.codex-work/skia/mo-skia.mjs','.codex-work/skia/mo-skia.wasm','.codex-work/text-component/index.js','.codex-work/raster-component/index.js',fontPath].map(entry);
const report={format:'musteroffice.text-page-runtime-parity/1',scope:'Current development Native worker/CLI and actual Rust WASM plus HarfBuzz/Skia components; exact metadata and all pixel bytes. No Office/WPS or product certification.',node:process.version,fixtures:entry(root+'/fixtures.json'),artifacts,cases:records,cliChecks,editRoundtrip,batchRequests:2,malformed,fault:{component:entry('.codex-work/harfbuzz/mo-hb.wasm'),failed,reused,rasterFailure,replacementVerified:true},counts:{parityRequests:records.length,rendered:records.filter(c=>c.pixels).length,cliRequests:cliChecks.length,overwriteRejections:cliChecks.filter(c=>c.overwriteRejected).length,workerFramingRejections:malformed.length}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report.counts));
