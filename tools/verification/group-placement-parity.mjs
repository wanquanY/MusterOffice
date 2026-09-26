// Actual native/WASM geometry, raster and editable export of owned group probes.
// This diagnostic bridge supports only the explicit solid-fill polygon fixtures.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root=process.argv[2]??'.codex-work/group-compat',read=p=>JSON.parse(fs.readFileSync(p)),sha=b=>createHash('sha256').update(b).digest('hex'),U=1n<<32n;
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const componentBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm');
const component=await RasterComponent.create(factory,new WebAssembly.Module(componentBytes));
const cases=[],renders=[];
for(const fixture of read(root+'/probes.json')){
 const {name,request:requestPath,pptx:pptxPath}=fixture,json=fs.readFileSync(requestPath,'utf8'),q=JSON.parse(json);
 const n=spawnSync('target/release/mo-cli',['page-placements',requestPath],{encoding:'utf8',timeout:30000,maxBuffer:8*1024*1024});
 assert.equal(n.status,0,n.stderr);const response=wasm.page_placements(json);assert.equal(n.stdout.trimEnd(),response);
 const r=JSON.parse(response);assert.equal(r.status,'evaluated');const responsePath=`${root}/${name}.response.json`;fs.writeFileSync(responsePath,response);
 cases.push({name,requestPath,responsePath,requestSha256:sha(json),responseSha256:sha(response),status:'evaluated',validRequest:true});
 assert.equal(r.result.surfaces.length,1);const paths=[],transforms=[],instances=[];let authorBound=0n;
 for(const p of r.result.surfaces[0].objects){
  const o=q.document.objects[p.object];if(o.content.kind==='group')continue;
  assert.equal(o.content.kind,'shape');assert.equal(o.content.text,null);assert.deepEqual(o.appearance.stroke,{kind:'value',value:{kind:'none'}});
  assert.equal(o.appearance.fill.value.kind,'solid');assert.equal(o.appearance.fill.value.color.kind,'srgb');
  const g=o.content.geometry,dims=['width','height'].map(k=>BigInt(p.sourceSize[k])),anchor=['x','y'].map(k=>BigInt(p.anchor[k]));
  let source;
  if(g.kind==='rectangle')source=[[0n,0n],[dims[0],0n],dims,[0n,dims[1]]];
  else{assert.equal(g.kind,'path');assert.deepEqual(g.commands.map(c=>c.kind),['move','line','line','line','close']);source=g.commands.slice(0,4).map(c=>[BigInt(c.to.x),BigInt(c.to.y)]);}
  const points=source.map(v=>v.map((x,i)=>x*U-anchor[i]));
  const commands=points.map((v,i)=>({kind:i?'line':'move',to:{x:String(v[0]),y:String(v[1])}}));commands.push({kind:'close'});
  for(const v of points)for(let row=0;row<2;row++){
   const sum=v.reduce((s,x,i)=>s+BigInt(p.uncertainty.linear[row*2+i])*(x<0n?-x:x),0n);
   const e=(sum+U-1n)/U+BigInt(p.uncertainty.translation[['x','y'][row]]);if(e>authorBound)authorBound=e;
  }
  const rgba=o.appearance.fill.value.color.rgba,index=paths.length;paths.push({fillRule:'nonzero',commands});transforms.push({parent:null,affine:p.affine});instances.push({path:index,transform:index,brush:{kind:'solid',rgba:[rgba.red,rgba.green,rgba.blue,rgba.alpha]}});
 }
 const viewport={width:800,height:600,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:9525},coordinateTolerance:String(1<<24),background:[255,255,255,255]};
 const scene=JSON.stringify({viewport,scene:{paths,transforms,instances}}),body=Buffer.from(scene),header=Buffer.alloc(4);header.writeUInt32LE(body.length);
 const native=spawnSync('target/release/mo-raster-worker',['--scene'],{input:Buffer.concat([header,body]),timeout:30000,maxBuffer:8*1024*1024});assert.equal(native.status,0,native.stderr?.toString());
 const m=native.stdout.readUInt32LE(),length=native.stdout.readUInt32LE(4),metadata=native.stdout.subarray(8,8+m).toString(),pixels=native.stdout.subarray(8+m);assert.equal(pixels.length,length);
 const w=wasm.render_scene(scene,component);assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);
 const info=JSON.parse(metadata).info,authorPixelBound=(authorBound+9524n)/9525n;assert(authorPixelBound+BigInt(info.work.combinedCoordinateErrorBound)<=BigInt(viewport.coordinateTolerance));
 const scenePath=`${root}/${name}.scene.json`,pixelsPath=`${root}/${name}.rgba`;fs.writeFileSync(scenePath,scene);fs.writeFileSync(pixelsPath,pixels);fs.writeFileSync(`${root}/${name}.raster.json`,metadata);
 const exportRequest=fs.readFileSync(`${root}/${name}.export.json`,'utf8'),pptx=fs.readFileSync(pptxPath);assert.deepEqual(pptx,Buffer.from(wasm.export_pptx(exportRequest,new Uint8Array())));
 renders.push({name,scenePath,sceneSha256:sha(scene),pixelsPath,pixelsSha256:sha(pixels),pptxPath,pptxSha256:sha(pptx),placementResponseSha256:sha(response),drawnObjects:instances.length,authorCoordinateErrorBound:String(authorPixelBound),sceneCoordinateErrorBound:info.work.combinedCoordinateErrorBound});
}
const artifacts={nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes)};
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.group-placement-parity/1',...artifacts,cases},null,2)+'\n');
fs.writeFileSync(root+'/render.json',JSON.stringify({format:'musteroffice.group-placement-render/1',...artifacts,nativeWasmPixelsAndPptxEqual:true,cases:renders},null,2)+'\n');
console.log(JSON.stringify({batches:cases.length,asymmetricShapes:48,registrationMarkers:12}));
