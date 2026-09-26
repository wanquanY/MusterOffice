/** Native linear gradient source pages and the sealed background corpus. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {RasterComponent} from '../../.codex-work/gradient-field/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-field/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/gradient-field',out=root+'/source-runtime';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/gradient-field/wasm-node/mo_wasm.js');
const skm=new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm'));
const hbm=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const old=JSON.parse(fs.readFileSync('.codex-work/compositing/source-parity.json')),records=[];
async function run(name,source,json,fonts,expected,previous){
 const j=Buffer.from(json),h=Buffer.alloc(12);h.writeUInt32LE(j.length);h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);
 const n=spawnSync('target/debug/mo-raster-worker',['--pptx-resource-page'],{input:Buffer.concat([h,j,source,fonts]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr.toString());const ml=n.stdout.readUInt32LE(),pl=n.stdout.readUInt32LE(4);assert.equal(n.stdout.length,8+ml+pl);
 const metadata=n.stdout.subarray(8,8+ml).toString(),pixels=n.stdout.subarray(8+ml),r=JSON.parse(metadata);
 const shaper=await ShapingComponent.create(hbFactory,hbm),component=await RasterComponent.create(skiaFactory,skm);
 let decodes=0,rasters=0,frame=null;
 const decoder={decodeImage(b){decodes++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
 const raster={rasterImages(f,b){rasters++;frame=f.slice();return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
 const w=wasm.render_pptx_resource_page(json,source,fonts,decoder,shaper,raster);
 assert.equal(w.metadata,metadata,name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,name);assert.equal(r.status,expected.status,name+': '+metadata);
 if(r.status==='rendered'){
  assert.equal(rasters,1);assert.equal(pixels.length,480000);assert.equal(r.info.page.scene.raster.sha256,sha(pixels));assert.equal(decodes,r.info.decodedImages.length);
  if(expected.pixels)assert.deepEqual(pixels,load(expected.pixels),name);
  if(expected.info)assert.deepEqual(r.info,JSON.parse(load(expected.info)),name);
 }else{assert.equal(rasters,0);assert.equal(pixels.length,0);}
 if(previous){assert.equal(metadata,load(previous.response).toString(),name);if(previous.pixels)assert.deepEqual(pixels,load(previous.pixels));}
 const p=out+'/'+name.replaceAll('/','-'),record={name,source:put(p+'.pptx',source),request:put(p+'.request.json',json),fonts:put(p+'.fonts.bin',fonts),response:put(p+'.response.json',metadata),status:r.status,decodes,rasters};
 if(frame)record.frame=put(p+'.frame',Buffer.from(frame.buffer));
 if(pixels.length)record.pixels=put(p+'.rgba',pixels);
 records.push(record);shaper.invalidate();component.invalidate();
}
const files=fs.readdirSync(root+'/native').filter(n=>n.endsWith('.pptx')).sort();assert.equal(files.length,17);
for(const file of files){
 const base=root+'/native/'+file.slice(0,-5),source=fs.readFileSync(base+'.pptx');
 const page=JSON.parse(fs.readFileSync(base+'.request.json')),q={profile:'drawingml-resource-page-q32-v1-draft',page,imageSource:'embeddedSnapshot',sampling:'nearest',fonts:null};
 await run('new/'+file.slice(0,-5),source,JSON.stringify(q),Buffer.alloc(0),{status:'rendered',pixels:entry(base+'.rgba'),info:entry(base+'.info.json')});
}
for(const name of JSON.parse(fs.readFileSync(root+'/invalid-source.json')).cases){
 await run('invalid/'+name,fs.readFileSync(root+'/invalid-source/'+name+'.pptx'),fs.readFileSync(root+'/invalid-source/'+name+'.json').toString(),Buffer.alloc(0),{status:'error'});
 assert.equal(records.at(-1).decodes,0,name);
}
const changed=[];
for(const c of old.cases){const upgraded=changed.includes(c.name);await run('prior/'+c.name,load(c.source),load(c.request).toString(),load(c.fonts),upgraded?{status:'rendered'}:c,upgraded?null:c);}
const positive=records.find(c=>c.name==='new/right'),negative=records.find(c=>c.name.endsWith('/inherited-stationary'));
const cliDir=fs.mkdtempSync(root+'/cli-'),output=cliDir+'/page.rgba',failure=cliDir+'/failure.rgba';
const cli=(c,p)=>spawnSync('target/debug/mo-cli',['render-pptx-resource-page',c.request.path,c.source.path,c.fonts.path,p],{env:{},timeout:60000,maxBuffer:80*1024*1024});
const created=cli(positive,output);assert.equal(created.status,0,created.stderr.toString());assert.deepEqual(fs.readFileSync(output),load(positive.pixels));
const before=entry(output),again=cli(positive,output);assert.notEqual(again.status,0);assert.deepEqual(entry(output),before);
const bad=cli(negative,failure);assert.equal(bad.status,0,bad.stderr.toString());assert(!fs.existsSync(failure));assert.equal(JSON.parse(bad.stdout).status,'error');
const report={format:'musteroffice.gradient-source-parity/1',pairedCalls:records.length,cases:records,previousUnchanged:old.cases.length-changed.length,previousNewlySupported:changed,previous:entry('.codex-work/compositing/source-parity.json'),cli:{pixels:before,createResponse:put(cliDir+'/created.json',created.stdout),overwriteExit:again.status,overwriteError:put(cliDir+'/overwrite.txt',again.stderr),failureResponse:put(cliDir+'/failure.json',bad.stdout),failureOutputAbsent:true}};
fs.writeFileSync(root+'/source-parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({paired:records.length,positive:records.filter(c=>c.status==='rendered').length,previousUnchanged:report.previousUnchanged,newlySupported:changed,cliVerified:true}));
