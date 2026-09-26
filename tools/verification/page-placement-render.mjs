// Diagnostic bridge for explicitly owned rectangle-only probes. This is not the
// production page painter: every content kind/style is asserted before lowering.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root='.codex-work/page-placement',read=p=>JSON.parse(fs.readFileSync(p)),sha=b=>createHash('sha256').update(b).digest('hex'),U=1n<<32n;
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const componentBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm'),component=await RasterComponent.create(factory,new WebAssembly.Module(componentBytes));
const defaults=read('fixtures/presentations/native-export/request.json').defaults,cases=[];
fs.writeFileSync(root+'/empty.bin',Buffer.alloc(0));
for(const c of read(root+'/parity.json').cases.filter(c=>c.name.startsWith('diagnostic-'))){
 const q=read(c.requestPath),result=read(c.responsePath).result;
 assert.equal(result.surfaces.length,1);const [group,placement]=result.surfaces[0].objects;
 assert.equal(q.document.objects[group.object].content.kind,'group');
 const o=q.document.objects[placement.object];assert.equal(o.content.kind,'shape');assert.deepEqual(o.content.geometry,{kind:'rectangle'});assert.equal(o.content.text,null);assert.deepEqual(o.appearance.stroke,{kind:'value',value:{kind:'none'}});
 const rgba=o.appearance.fill.value.color.rgba;assert.equal(o.appearance.fill.value.kind,'solid');assert.equal(o.appearance.fill.value.color.kind,'srgb');
 const dims=['width','height'].map(k=>BigInt(placement.sourceSize[k])*U),anchor=['x','y'].map(k=>BigInt(placement.anchor[k]));
 const corners=[[0n,0n],[dims[0],0n],dims,[0n,dims[1]]].map(p=>p.map((v,i)=>v-anchor[i]));
 const commands=corners.map((p,i)=>({kind:i?'line':'move',to:{x:String(p[0]),y:String(p[1])}}));commands.push({kind:'close'});
 const v={width:800,height:600,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:9525},coordinateTolerance:String(1<<24),background:[255,255,255,255]};
 const request={viewport:v,scene:{paths:[{fillRule:'nonzero',commands}],transforms:[{parent:null,affine:placement.affine}],instances:[{path:0,transform:0,brush:{kind:'solid',rgba:[rgba.red,rgba.green,rgba.blue,rgba.alpha]}}]}};
 let authorBound=0n;for(const p of corners)for(let row=0;row<2;row++){
  const sum=p.reduce((s,x,i)=>s+BigInt(placement.uncertainty.linear[row*2+i])*(x<0n?-x:x),0n);
  const error=(sum+U-1n)/U+BigInt(placement.uncertainty.translation[['x','y'][row]]);if(error>authorBound)authorBound=error;
 }
 const authorPixelBound=(authorBound+9524n)/9525n,json=JSON.stringify(request),b=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(b.length);
 const n=spawnSync('target/release/mo-raster-worker',['--scene'],{input:Buffer.concat([h,b]),timeout:30000,maxBuffer:8*1024*1024});assert.equal(n.status,0,n.stderr?.toString());
 const m=n.stdout.readUInt32LE(),length=n.stdout.readUInt32LE(4),metadata=n.stdout.subarray(8,8+m).toString(),pixels=n.stdout.subarray(8+m);assert.equal(pixels.length,length);
 const w=wasm.render_scene(json,component);assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);const info=JSON.parse(metadata).info;
 assert(authorPixelBound+BigInt(info.work.combinedCoordinateErrorBound)<=BigInt(v.coordinateTolerance));
 const stem=`${root}/${c.name}`,scenePath=stem+'.scene.json',pixelsPath=stem+'.rgba';fs.writeFileSync(scenePath,json);fs.writeFileSync(pixelsPath,pixels);fs.writeFileSync(stem+'.raster.json',metadata);
 const exportRequest={document:q.document,defaults,resourceBindings:[]};fs.writeFileSync(stem+'.export.json',JSON.stringify(exportRequest));fs.rmSync(stem+'.pptx',{force:true});
 const exported=spawnSync('target/release/mo-cli',['pptx-export',stem+'.export.json',root+'/empty.bin',stem+'.pptx'],{encoding:'utf8',timeout:30000});assert.equal(exported.status,0,exported.stderr);
 const pptx=fs.readFileSync(stem+'.pptx');assert.deepEqual(pptx,Buffer.from(wasm.export_pptx(JSON.stringify(exportRequest),new Uint8Array())));
 cases.push({name:c.name,placementResponseSha256:c.responseSha256,scenePath,sceneSha256:sha(json),pixelsPath,pixelsSha256:sha(pixels),authorCoordinateErrorBound:String(authorPixelBound),sceneCoordinateErrorBound:info.work.combinedCoordinateErrorBound,pptxPath:stem+'.pptx',pptxSha256:sha(pptx)});
}
const report={format:'musteroffice.page-placement-render/1',scope:'Four owned rectangle-only transform probes; diagnostic bridge explicitly asserts content/style; not complete page rendering.',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),nativeWasmPixelsAndPptxEqual:true,cases};
fs.writeFileSync(root+'/render.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({renderedAndExported:cases.length}));
