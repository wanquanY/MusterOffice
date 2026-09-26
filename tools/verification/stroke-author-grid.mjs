// Owned native-editable style grid for target-application canvas observation.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root='.codex-work/stroke-author/grid';fs.mkdirSync(root,{recursive:true});
const read=p=>JSON.parse(fs.readFileSync(p)),sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>({path:p,byteLength:fs.statSync(p).size,sha256:sha(fs.readFileSync(p))});
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const q=read('.codex-work/stroke-author/external/stroke-author-flat-round.page.json');
const d=q.page.document,template=structuredClone(d.objects['shape:1']);d.objects={};d.slides['slide:1'].objects=[];
const styles=[];
for(const cap of ['flat','round','square'])for(const kind of ['round','bevel','miter'])styles.push({cap,join:{kind,...(kind==='miter'?{limit:400000}:{})}});
for(const limit of [0,100000,800000])styles.push({cap:'flat',join:{kind:'miter',limit}});
const mapping=[];
for(const [i,style] of styles.entries()){
 const o=structuredClone(template);o.id=`probe-${i+1}`;
 o.transform.origin={x:String((45+200*(i%4))*9525),y:String((25+200*Math.floor(i/4))*9525)};
 o.transform.size={width:String(110*9525),height:String(140*9525)};
 Object.assign(o.appearance.stroke.value,style,{width:String(24*9525)});
 d.objects[o.id]=o;d.slides['slide:1'].objects.push(o.id);
 mapping.push({object:o.id,row:Math.floor(i/4),column:i%4,...style});
}
const pagePath=root+'/grid.page.json',pixelsPath=root+'/grid.rgba';fs.writeFileSync(pagePath,JSON.stringify(q));fs.rmSync(pixelsPath,{force:true});
const render=spawnSync('target/release/mo-cli',['render-page',pagePath,pixelsPath],{encoding:'utf8',timeout:30000});assert.equal(render.status,0,render.stderr);assert.equal(JSON.parse(render.stdout).status,'rendered');
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
const wr=wasm.render_page(JSON.stringify(q),component);assert.equal(wr.metadata,render.stdout.trimEnd());assert.deepEqual(Buffer.from(wr.take_pixels()),fs.readFileSync(pixelsPath));
const er=read('.codex-work/stroke-author/external/stroke-author-flat-round.export.json');er.document=d;
const exportPath=root+'/grid.export.json',pptxPath=root+'/stroke-author-grid.pptx',json=JSON.stringify(er);fs.writeFileSync(exportPath,json);fs.writeFileSync(root+'/empty.bin','');fs.rmSync(pptxPath,{force:true});
const result=spawnSync('target/release/mo-cli',['pptx-export',exportPath,root+'/empty.bin',pptxPath],{encoding:'utf8',timeout:30000});assert.equal(result.status,0,result.stderr);assert.equal(JSON.parse(result.stdout).status,'inspected');
assert.deepEqual(Buffer.from(wasm.export_pptx(json,new Uint8Array())),fs.readFileSync(pptxPath));
const plan=wasm.compile_page(JSON.stringify(q));assert.equal(JSON.parse(plan).status,'compiled');fs.writeFileSync(root+'/grid.plan.json',plan);
fs.writeFileSync(root+'/inputs.json',JSON.stringify({format:'musteroffice.stroke-author-grid/1',mapping,page:entry(pagePath),export:entry(exportPath),pptx:entry(pptxPath),plan:entry(root+'/grid.plan.json'),pixels:entry(pixelsPath),nativeCli:entry('target/release/mo-cli'),rustWasm:entry('.codex-work/wasm-node/mo_wasm_bg.wasm'),nativeWasmPptxEqual:true,nativeWasmPixelsAndMetadataEqual:true},null,2)+'\n');
console.log(JSON.stringify({nativeEditableObjects:styles.length,pptxPath}));
