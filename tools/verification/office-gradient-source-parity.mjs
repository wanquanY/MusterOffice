/** Office gradient source pages and prior source corpus with explicit frame deltas. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {RasterComponent} from '../../.codex-work/office-gradient/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/office-gradient/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/office-gradient',out=root+'/source-runtime';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/office-gradient/wasm-node/mo_wasm.js');
const skm=new WebAssembly.Module(fs.readFileSync('.codex-work/office-gradient/component/mo-skia.wasm'));
const hbm=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const old=JSON.parse(fs.readFileSync('.codex-work/paint-admission/source-parity.json')),records=[];
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
const files=fs.readdirSync(root+'/native').filter(n=>n.endsWith('.pptx')).sort();assert.equal(files.length,10);
for(const file of files){
 const name=file.slice(0,-5),base=root+'/native/'+name,source=fs.readFileSync(base+'.pptx');
 const page=JSON.parse(fs.readFileSync(base+'.request.json')); const expected={status:'rendered',pixels:entry(base+'.rgba'),info:entry(base+'.info.json')};
 const q={profile:'drawingml-resource-page-q32-v1-draft',page,imageSource:'embeddedSnapshot',sampling:'nearest',fonts:null};
 await run('new/'+name,source,JSON.stringify(q),Buffer.alloc(0),expected);
 assert.equal(records.at(-1).decodes,0);
}

const changed=old.cases.filter(c=>c.name.startsWith('prior/new/') && ['alpha','background-window','background','down','group-inherited-control','group-inherited','left','right','rotated-ellipse','scaled-225','scaled-30','tile-none','tile-x','tile-xy','unscaled-30','up'].includes(c.name.slice(10))).map(c=>c.name);
assert.equal(changed.length,16);
function verifyUpgrade(c, now) {
 const bytes=load(c.frame),before=new Uint32Array(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.length));
 assert.equal(before[1],9);before[1]=10;
 let pos=14;for(let j=0;j<before[5];j++)pos+=2+7*before[pos+1];pos+=4*before[8];let upgrades=0;
 const f=n=>new Float32Array(new Uint32Array([n]).buffer)[0];
 for(let j=0;j<before[9];j++){
  const header=before[pos]===2?16:9,n=before[pos+4],start=pos+header,end=start+5*(n-1);
  const eligible=(n===2||n===3)&&f(before[start])===0&&f(before[end])===1&&(n===2||f(before[start+5])>0&&f(before[start+5])<1&&[1,2,3,4].every(k=>before[start+k]===before[end+k]));
  if(eligible){assert.equal(before[pos+2],0);before[pos+2]=2;upgrades++;}pos+=header+5*n;
 }
 assert(upgrades>0);assert.deepEqual(Buffer.from(before.buffer),load(now.frame));
 const a=JSON.parse(load(c.response)),b=JSON.parse(load(now.response));
 for(const v of [a,b]) for(const key of ['profile','sha256','frameSha256']) delete v.info.page.scene.raster[key];
 assert.deepEqual(a,b);assert.notEqual(c.pixels.sha256,now.pixels.sha256);
}
for(const c of old.cases){const upgraded=changed.includes(c.name);await run('prior/'+c.name,load(c.source),load(c.request).toString(),load(c.fonts),upgraded?{status:'rendered'}:c,upgraded?null:c);if(upgraded)verifyUpgrade(c,records.at(-1));}
const positive=records.find(c=>c.name==='new/pair'),negative=records.find(c=>c.name.endsWith('/over-limit'));
const cliDir=fs.mkdtempSync(root+'/cli-'),output=cliDir+'/page.rgba',failure=cliDir+'/failure.rgba';
const cli=(c,p)=>spawnSync('target/debug/mo-cli',['render-pptx-resource-page',c.request.path,c.source.path,c.fonts.path,p],{env:{},timeout:60000,maxBuffer:80*1024*1024});
const created=cli(positive,output);assert.equal(created.status,0,created.stderr.toString());assert.deepEqual(fs.readFileSync(output),load(positive.pixels));
const before=entry(output),again=cli(positive,output);assert.notEqual(again.status,0);assert.deepEqual(entry(output),before);
const bad=cli(negative,failure);assert.equal(bad.status,0,bad.stderr.toString());assert(!fs.existsSync(failure));assert.equal(JSON.parse(bad.stdout).status,'error');
const report={format:'musteroffice.office-gradient-source-parity/1',pairedCalls:records.length,cases:records,previousUnchanged:old.cases.length-changed.length,previousColorCorrected:changed,previous:entry('.codex-work/paint-admission/source-parity.json'),cli:{pixels:before,createResponse:put(cliDir+'/created.json',created.stdout),overwriteExit:again.status,overwriteError:put(cliDir+'/overwrite.txt',again.stderr),failureResponse:put(cliDir+'/failure.json',bad.stdout),failureOutputAbsent:true}};
fs.writeFileSync(root+'/source-parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({paired:records.length,positive:records.filter(c=>c.status==='rendered').length,previousUnchanged:report.previousUnchanged,newlySupported:changed,cliVerified:true}));
