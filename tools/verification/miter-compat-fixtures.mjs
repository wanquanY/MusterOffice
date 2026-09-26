// Controlled owned documents isolate miter limit, pen width and path coordinate units.
// These are compatibility probes, not accepted target-application render profiles.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {base,shape,solid,rgba} from './page-render-fixtures.mjs';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root='.codex-work/miter-compat',E=9525,read=p=>JSON.parse(fs.readFileSync(p));
fs.mkdirSync(root,{recursive:true});fs.writeFileSync(root+'/empty.bin','');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>({path:p,byteLength:fs.statSync(p).size,sha256:sha(fs.readFileSync(p))});
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
const defaults=read('fixtures/presentations/native-export/request.json').defaults;
const studies={
 limits:[0,100000,150000,200000,300000,400000,500000,600000,800000,1200000,2000000,4000000].map(limit=>({limit,width:24,units:E})),
 widths:[4,12,24,48].flatMap(width=>[200000,400000,800000].map(limit=>({width,limit,units:E}))),
 coordinates:[1,10,9525,10000].flatMap(units=>[200000,400000,800000].map(limit=>({units,limit,width:24}))),
 thinlimits:[4,12].flatMap(width=>[0,100000,150000,200000,400000,800000].map(limit=>({width,limit,units:E}))),
 angles:[6,12,24,48].flatMap(width=>[20,30,40].map(spread=>({width,spread,limit:400000,units:E}))),
};
const files=[];
for(const [name,cases] of Object.entries(studies)){
 const q=base(),template=structuredClone(shape(q)),d=q.page.document;d.pageSize={width:String(800*E),height:String(600*E)};
 q.viewport.scale={numerator:1,denominator:E};d.slides['slide:1'].background=solid(rgba(255,255,255));d.objects={};d.slides['slide:1'].objects=[];
 const mapping=[];
 for(const [i,{limit,width,units,spread=40}] of cases.entries()){
  const o=structuredClone(template),row=Math.floor(i/4),column=i%4;o.id=`${name}-${i}`;
  o.transform.origin={x:String((45+200*column)*E),y:String((25+200*row)*E)};
  o.transform.size={width:String(110*E),height:String(140*E)};
  o.content.geometry={kind:'path',viewport:{width:String(100*units),height:String(100*units)},commands:[['move',50-spread,80],['line',50,10],['line',50+spread,80]].map(([kind,x,y])=>({kind,to:{x:String(x*units),y:String(y*units)}}))};
  o.appearance.fill={kind:'value',value:{kind:'none'}};
  o.appearance.stroke={kind:'value',value:{kind:'solid',width:String(width*E),cap:'flat',join:{kind:'miter',limit},color:{kind:'srgb',rgba:rgba(180,40,70)}}};
  d.objects[o.id]=o;d.slides['slide:1'].objects.push(o.id);mapping.push({object:o.id,row,column,limit,width,units,spread});
 }
 const pagePath=`${root}/${name}.page.json`,pixelsPath=`${root}/${name}.rgba`,pptxPath=`${root}/miter-${name}.pptx`,exportPath=`${root}/${name}.export.json`;
 fs.writeFileSync(pagePath,JSON.stringify(q));fs.rmSync(pixelsPath,{force:true});
 const native=spawnSync('target/release/mo-cli',['render-page',pagePath,pixelsPath],{encoding:'utf8',timeout:30000});assert.equal(native.status,0,native.stderr);assert.equal(JSON.parse(native.stdout).status,'rendered',name);
 const w=wasm.render_page(JSON.stringify(q),component);assert.equal(w.metadata,native.stdout.trimEnd());assert.deepEqual(Buffer.from(w.take_pixels()),fs.readFileSync(pixelsPath));
 const json=JSON.stringify({document:d,defaults,resourceBindings:[]});fs.writeFileSync(exportPath,json);fs.rmSync(pptxPath,{force:true});
 const exportResult=spawnSync('target/release/mo-cli',['pptx-export',exportPath,root+'/empty.bin',pptxPath],{encoding:'utf8',timeout:30000});assert.equal(exportResult.status,0,exportResult.stderr);assert.equal(JSON.parse(exportResult.stdout).status,'inspected');
 assert.deepEqual(Buffer.from(wasm.export_pptx(json,new Uint8Array())),fs.readFileSync(pptxPath));
 const plan=wasm.compile_page(JSON.stringify(q));assert.equal(JSON.parse(plan).status,'compiled');fs.writeFileSync(`${root}/${name}.plan.json`,plan);
 files.push({name,mapping,page:entry(pagePath),pixels:entry(pixelsPath),export:entry(exportPath),pptx:entry(pptxPath),plan:entry(`${root}/${name}.plan.json`)});
}
const report={format:'musteroffice.miter-compat-inputs/1',files,nativeCli:entry('target/release/mo-cli'),rustWasm:entry('.codex-work/wasm-node/mo_wasm_bg.wasm'),rasterWasm:entry('.codex-work/skia/mo-skia.wasm'),nativeWasmPptxAndPixelsEqual:true};
fs.writeFileSync(root+'/inputs.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({pages:files.length,objects:files.reduce((n,c)=>n+c.mapping.length,0)}));
