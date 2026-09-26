// Same author objects drive page raster and native editable PPTX export.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const root='.codex-work/page-render/external',read=p=>JSON.parse(fs.readFileSync(p));fs.mkdirSync(root,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js'),defaults=read('fixtures/presentations/native-export/request.json').defaults;
const sha=b=>createHash('sha256').update(b).digest('hex');
function emu(value){if(Array.isArray(value))return value.map(emu);if(value&&typeof value==='object')return Object.fromEntries(Object.entries(value).map(([k,v])=>[k,['x','y','width','height','radius'].includes(k)&&typeof v==='string'?String(BigInt(v)*9525n):emu(v)]));return value;}
const cases=[];fs.writeFileSync(root+'/empty.bin','');
for(const name of ['rectangle-2700000','ellipse-0','ellipse-2700000','ellipse--2147483648','roundRectangle-0','roundRectangle--8100000','round-radius-80','custom-curves']){
 const q=read(`.codex-work/page-render/${name}.request.json`);q.page.document=emu(q.page.document);q.viewport.scale={numerator:1,denominator:9525};
 const requestPath=`${root}/${name}.page.json`,pixelsPath=`${root}/${name}.rgba`;fs.writeFileSync(requestPath,JSON.stringify(q));fs.rmSync(pixelsPath,{force:true});
 const r=spawnSync('target/release/mo-cli',['render-page',requestPath,pixelsPath],{encoding:'utf8',timeout:30000});assert.equal(r.status,0,r.stderr);const response=JSON.parse(r.stdout);assert.equal(response.status,'rendered');
 const exportRequest={document:q.page.document,defaults,resourceBindings:[]},json=JSON.stringify(exportRequest),exportPath=`${root}/${name}.export.json`,pptxPath=`${root}/${name}.pptx`;fs.writeFileSync(exportPath,json);fs.rmSync(pptxPath,{force:true});
 const e=spawnSync('target/release/mo-cli',['pptx-export',exportPath,root+'/empty.bin',pptxPath],{encoding:'utf8',timeout:30000});assert.equal(e.status,0,e.stderr);const bytes=Buffer.from(wasm.export_pptx(json,new Uint8Array()));assert.deepEqual(bytes,fs.readFileSync(pptxPath));
 const plan=wasm.compile_page(JSON.stringify(q));assert.equal(JSON.parse(plan).status,'compiled');const planPath=`${root}/${name}.plan.json`;fs.writeFileSync(planPath,plan);
 cases.push({name,requestPath,requestSha256:sha(fs.readFileSync(requestPath)),planPath,planSha256:sha(plan),pptxPath,pptxSha256:sha(bytes),pixelsPath,pixelsSha256:sha(fs.readFileSync(pixelsPath)),response});
}
const report={format:'musteroffice.page-render-external-inputs/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,nativeWasmPptxEqual:true};
fs.writeFileSync(root+'/inputs.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({editablePptx:cases.length}));
