/** Actual source-page boundary: new group cases plus all previous requests. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import skiaFactory from '../../.codex-work/clips/component/mo-skia.mjs';
const root='.codex-work/group-image',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/group-image/wasm-node/mo_wasm.js');
const hbm=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const skm=new WebAssembly.Module(fs.readFileSync('.codex-work/clips/component/mo-skia.wasm'));
const fixtures=JSON.parse(fs.readFileSync(root+'/fixtures.json')),old=JSON.parse(fs.readFileSync('.codex-work/resource-page/parity.json'));
const records=[];
function request(source){return {profile:'drawingml-resource-page-q32-v1-draft',page:{expectedSourceSha256:sha(source),slide:'/ppt/slides/slide1.xml',profile:'drawingml-static-solid-page-v1-draft',colorContext:{systemColors:{},placeholder:null},viewport:{width:400,height:300,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:4000},coordinateTolerance:'16777216',background:[255,255,255,255]}},imageSource:'embeddedSnapshot',sampling:'nearest',fonts:null};}
async function run(name,source,json,fonts,expected,prior){
 const j=Buffer.from(json),h=Buffer.alloc(12);h.writeUInt32LE(j.length);h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);
 const n=spawnSync('target/debug/mo-raster-worker',['--pptx-resource-page'],{input:Buffer.concat([h,j,source,fonts]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr.toString());const ml=n.stdout.readUInt32LE(),pl=n.stdout.readUInt32LE(4);assert.equal(n.stdout.length,8+ml+pl);
 const metadata=n.stdout.subarray(8,8+ml).toString(),pixels=n.stdout.subarray(8+ml),r=JSON.parse(metadata);
 const shaper=await ShapingComponent.create(hbFactory,hbm),component=await RasterComponent.create(skiaFactory,skm);
 let decodes=0,rasters=0;
 const decoder={decodeImage(b){decodes++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
 const raster={rasterImages(f,b){rasters++;return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
 const w=wasm.render_pptx_resource_page(json,source,fonts,decoder,shaper,raster);
 assert.equal(w.metadata,metadata,name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,name);
 assert.equal(r.status,expected.status,name+': '+metadata);
 if(r.status==='rendered'){
  assert.equal(rasters,1);assert.equal(pixels.length,480000);assert.equal(r.info.page.scene.raster.sha256,sha(pixels));
  assert.equal(decodes,r.info.decodedImages.length);
  if(expected.pixels)assert.deepEqual(pixels,load(expected.pixels),name);
  if(expected.info)assert.deepEqual(r.info,JSON.parse(load(expected.info)),name);
 }else{
  assert.equal(rasters,0);assert.equal(pixels.length,0);
  if(expected.code)assert.equal(r.error.error.code,expected.code);
  if(expected.imageIssue)assert.equal(r.error.image.kind,expected.imageIssue);
 }
 if(name.startsWith('new/'))assert.equal(decodes,r.status==='rendered'?1:0,name);
 if(prior){assert.equal(metadata,load(prior.response).toString(),name);if(prior.pixels)assert.deepEqual(pixels,load(prior.pixels),name);}
 shaper.invalidate();component.invalidate();
 const prefix=out+'/'+name.replaceAll('/','-'),record={name,source:put(prefix+'.pptx',source),request:put(prefix+'.request.json',json),fonts:put(prefix+'.fonts.bin',fonts),response:put(prefix+'.response.json',metadata),status:r.status,decodes,rasters};
 if(pixels.length)record.pixels=put(prefix+'.rgba',pixels);
 records.push(record);
}
for(const c of fixtures.cases){const s=load(c.source),q=request(s);q.imageSource=c.imageSource??q.imageSource;await run('new/'+c.name,s,JSON.stringify(q),Buffer.alloc(0),c);}
for(const c of old.cases){
 const changed=c.name==='group-fill';
 await run('previous/'+c.name,load(c.source),load(c.request).toString(),load(c.fonts),changed?{status:'rendered'}:c,changed?undefined:c);
}
// Equivalent direct declarations are a separate native package, not a
// renderer flag or a replacement scene. Compare all pixels after both APIs.
const equivalent=[];
for(const c of fixtures.cases.filter(c=>c.status==='rendered'&&!c.name.endsWith('-explicit'))){
 const a=records.find(r=>r.name==='new/'+c.name),b=records.find(r=>r.name==='new/'+c.name+'-explicit');
 assert.deepEqual(load(a.pixels),load(b.pixels),c.name);equivalent.push({name:c.name,inherited:a.pixels,explicit:b.pixels});
}
const positive=records.find(c=>c.name==='new/nested-oriented'),negative=records.find(c=>c.name==='new/inherited-stationary');
const cliDir=fs.mkdtempSync(root+'/cli-'),output=cliDir+'/page.rgba',failure=cliDir+'/failure.rgba';
const cli=(c,p)=>spawnSync('target/debug/mo-cli',['render-pptx-resource-page',c.request.path,c.source.path,c.fonts.path,p],{env:{},timeout:60000,maxBuffer:80*1024*1024});
const created=cli(positive,output);assert.equal(created.status,0,created.stderr.toString());assert.deepEqual(fs.readFileSync(output),load(positive.pixels));
const before=entry(output),again=cli(positive,output);assert.notEqual(again.status,0);assert.deepEqual(entry(output),before);
const bad=cli(negative,failure);assert.equal(bad.status,0,bad.stderr.toString());assert(!fs.existsSync(failure));assert.equal(JSON.parse(bad.stdout).status,'error');
const report={format:'musteroffice.group-image-parity/1',pairedCalls:records.length,cases:records,equivalent,previousUnchanged:32,previousNewlySupported:['group-fill'],fixtures:entry(root+'/fixtures.json'),previous:entry('.codex-work/resource-page/parity.json'),cli:{pixels:before,createResponse:put(cliDir+'/created.json',created.stdout),overwriteExit:again.status,overwriteError:put(cliDir+'/overwrite.txt',again.stderr),failureResponse:put(cliDir+'/failure.json',bad.stdout),failureOutputAbsent:true}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({pairedCalls:records.length,successful:records.filter(c=>c.status==='rendered').length,equivalent:equivalent.length,previousUnchanged:32,cliVerified:true}));
