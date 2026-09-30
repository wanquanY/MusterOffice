// Rust owns all style selection and drawing instructions. This harness only
// supplies a viewport to the existing scene renderer; no chart paint is rebuilt.
import fs from 'node:fs/promises';
import path from 'node:path';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const [configFile,output]=process.argv.slice(2);
assert.ok(output,'usage: node chart-plot-parity.mjs <config.json> <new-report.json>');
const config=JSON.parse(await fs.readFile(configFile,'utf8'));
const wasm=createRequire(import.meta.url)(path.resolve(config.wasmModule));
const sha=b=>createHash('sha256').update(b).digest('hex');
const {default:factory}=await import(pathToFileURL(path.resolve(config.componentModule)));
const {RasterComponent}=await import(pathToFileURL(path.resolve(config.adapterModule)));
const component=await RasterComponent.create(factory,new WebAssembly.Module(await fs.readFile(config.componentWasm)));
const records=[],unit=1n<<32n;
for(const item of config.cases){
  const raw=await fs.readFile(item.request,'utf8'),source=await fs.readFile(item.source),q=JSON.parse(raw);
  const response=JSON.parse(execFileSync(config.cli,['compile-pptx-chart-plot',item.request,item.source],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024}));
  assert.deepEqual(JSON.parse(wasm.compile_pptx_chart_plot(raw,source)),response,item.name);
  assert.equal(response.status,item.expectedStatus,item.name);
  const record={name:item.name,requestSha256:sha(raw),sourceSha256:sha(source),response};
  if(response.status==='compiled'){
    assert.equal(q.geometry.outerRadius,String(1_000_000n*unit));
    assert.deepEqual(q.geometry.center,{x:'0',y:'0'});
    const request={viewport:{width:512,height:512,origin:{x:String(-1_280_000n*unit),y:String(-1_280_000n*unit)},scale:{numerator:1,denominator:5000},coordinateTolerance:String(1<<24),background:[0,0,0,0]},scene:response.plot.scene};
    const json=JSON.stringify(request),header=Buffer.alloc(4);header.writeUInt32LE(Buffer.byteLength(json));
    const wire=execFileSync(config.worker,['--scene'],{input:Buffer.concat([header,Buffer.from(json)]),timeout:30000,maxBuffer:16*1024*1024,env:{}});
    const n=wire.readUInt32LE(),metadata=JSON.parse(wire.subarray(8,8+n)),pixels=wire.subarray(8+n);
    assert.equal(wire.readUInt32LE(4),pixels.length);assert.equal(metadata.status,'rendered');
    const browser=wasm.render_scene(json,component);
    assert.deepEqual(JSON.parse(browser.metadata),metadata);assert.deepEqual(Buffer.from(browser.take_pixels()),pixels);
    const upstream=BigInt(response.plot.coordinateErrorBound);
    const bound=(upstream+4999n)/5000n+BigInt(metadata.info.work.combinedCoordinateErrorBound);
    assert.ok(bound<=BigInt(request.viewport.coordinateTolerance));
    assert.equal(metadata.info.raster.work.strokeDraws,request.scene.instances.filter(i=>i.stroke).length);
    await fs.writeFile(path.join(path.dirname(output),item.name+'.rgba'),pixels,{flag:'wx'});
    record.raster={width:512,height:512,metadata,pixelSha256:sha(pixels),combinedCoordinateErrorPixelsQ32:String(bound),completePixelsAndMetadataEqual:true};
  }
  assert.equal(sha(await fs.readFile(item.source)),record.sourceSha256);
  records.push(record);
}
await fs.writeFile(output,JSON.stringify({profile:'source-chart-declared-plot-parity/1',cases:records,completeResponsesEqual:true,fullChartRendered:false,officeWpsProven:false},null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({cases:records.length,compiled:records.filter(r=>r.response.status==='compiled').length,rendered:records.filter(r=>r.raster).length,completeResponsesEqual:true}));
