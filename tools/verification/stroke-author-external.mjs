// Same author documents -> real Native/WASM PPTX, compilation and page pixels.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root='.codex-work/stroke-author/external',read=p=>JSON.parse(fs.readFileSync(p));fs.mkdirSync(root,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
const defaults=read('fixtures/presentations/native-export/request.json').defaults;
const sha=b=>createHash('sha256').update(b).digest('hex');
function emu(v){
 if(Array.isArray(v))return v.map(emu);
 if(v&&typeof v==='object')return Object.fromEntries(Object.entries(v).map(([k,x])=>[k,['x','y','width','height','radius'].includes(k)&&typeof x==='string'?String(BigInt(x)*9525n):emu(x)]));
 return v;
}
const cases=[];fs.writeFileSync(root+'/empty.bin','');
for(const c of read('.codex-work/page-render/parity.json').cases){
 if(!c.name.startsWith('stroke-author-')||c.status!=='rendered')continue;
 const name=c.name,q=read(c.requestPath);q.page.document=emu(q.page.document);q.viewport.scale.denominator*=9525;
 // Theme fallback must be the same explicit context for native export and page.
 const exportDefaults=structuredClone(defaults);
 Object.assign(exportDefaults.themeColors,q.defaults.themeColors);
 exportDefaults.pageBackground=q.defaults.pageBackground;
 const requestPath=`${root}/${name}.page.json`,pixelsPath=`${root}/${name}.rgba`;fs.writeFileSync(requestPath,JSON.stringify(q));fs.rmSync(pixelsPath,{force:true});
 const r=spawnSync('target/release/mo-cli',['render-page',requestPath,pixelsPath],{encoding:'utf8',timeout:30000});assert.equal(r.status,0,r.stderr);assert.equal(JSON.parse(r.stdout).status,'rendered',name);
 const wr=wasm.render_page(JSON.stringify(q),component);assert.equal(wr.metadata,r.stdout.trimEnd(),name+' page metadata');assert.deepEqual(Buffer.from(wr.take_pixels()),fs.readFileSync(pixelsPath),name+' page pixels');
 const exportRequest={document:q.page.document,defaults:exportDefaults,resourceBindings:[]},json=JSON.stringify(exportRequest),exportPath=`${root}/${name}.export.json`,pptxPath=`${root}/${name}.pptx`;fs.writeFileSync(exportPath,json);fs.rmSync(pptxPath,{force:true});
 const e=spawnSync('target/release/mo-cli',['pptx-export',exportPath,root+'/empty.bin',pptxPath],{encoding:'utf8',timeout:30000});assert.equal(e.status,0,e.stderr);assert.equal(JSON.parse(e.stdout).status,'inspected',name);
 const bytes=Buffer.from(wasm.export_pptx(json,new Uint8Array()));assert.deepEqual(bytes,fs.readFileSync(pptxPath));
 const plan=wasm.compile_page(JSON.stringify(q));assert.equal(JSON.parse(plan).status,'compiled');const planPath=`${root}/${name}.plan.json`;fs.writeFileSync(planPath,plan);
 cases.push({name,requestPath,requestSha256:sha(fs.readFileSync(requestPath)),exportPath,exportSha256:sha(json),planPath,planSha256:sha(plan),pptxPath,pptxSha256:sha(bytes),pixelsPath,pixelsSha256:sha(fs.readFileSync(pixelsPath)),byteLength:bytes.length});
}
const report={format:'musteroffice.stroke-author-external-inputs/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,nativeWasmPptxEqual:true,nativeWasmPixelsAndMetadataEqual:true};
fs.writeFileSync(root+'/inputs.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({editablePptx:cases.length}));
