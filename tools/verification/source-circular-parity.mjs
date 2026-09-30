import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const [configFile, output]=process.argv.slice(2);
assert.ok(output,'usage: node source-circular-parity.mjs <config.json> <new-report.json>');
const config=JSON.parse(await fs.readFile(configFile,'utf8'));
const wasm=createRequire(import.meta.url)(path.resolve(config.wasmModule));
const sha=b=>createHash('sha256').update(b).digest('hex');
const U=1n<<32n,point=(x,y)=>({x:String(BigInt(x)*U),y:String(BigInt(y)*U)});
const {default:factory}=await import(pathToFileURL(path.resolve(config.componentModule)));
const {RasterComponent}=await import(pathToFileURL(path.resolve(config.adapterModule)));
const component=await RasterComponent.create(factory,new WebAssembly.Module(await fs.readFile(config.componentWasm)));
const palette=[[41,91,160,255],[19,166,179,255],[240,163,49,255],[177,80,151,255]];
const records=[];
for(const item of config.cases){
  const raw=await fs.readFile(item.request,'utf8'),source=await fs.readFile(item.source),q=JSON.parse(raw);
  const native=JSON.parse(execFileSync(config.cli,['compile-pptx-chart-geometry',item.request,item.source],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024}));
  assert.deepEqual(JSON.parse(wasm.compile_pptx_chart_geometry(raw,source)),native,item.name);
  assert.equal(native.status,item.expectedStatus,item.name);
  if(item.expectedCode)assert.equal(native.error.code,item.expectedCode,item.name);
  const record={name:item.name,requestSha256:sha(raw),sourceSha256:sha(source),response:native};
  if(item.render){
    assert.equal(native.status,'compiled');assert.deepEqual(q.center,point(0,0));assert.equal(q.outerRadius,String(1000n*U));
    const paths=[],draws=[];let upstream=0n;
    for(const series of native.geometry.series){
      upstream=upstream>BigInt(series.coordinateErrorBound)?upstream:BigInt(series.coordinateErrorBound);
      for(const p of series.geometry.paths){
        if(!p.commands.length)continue;
        draws.push({path:paths.length,origin:point(0,0),brush:{kind:'solid',rgba:palette[p.pointIndex%4]}});
        paths.push({fillRule:series.geometry.fillRule,commands:p.commands});
      }
    }
    const request={viewport:{width:512,height:512,origin:point(-1280,-1280),scale:{numerator:1,denominator:5},coordinateTolerance:String(1<<24),background:[0,0,0,0]},paths,draws};
    const json=JSON.stringify(request),header=Buffer.alloc(4);header.writeUInt32LE(Buffer.byteLength(json));
    const reply=execFileSync(config.worker,[],{input:Buffer.concat([header,Buffer.from(json)]),timeout:30000,maxBuffer:80*1024*1024,env:{}});
    const len=reply.readUInt32LE(),metadata=JSON.parse(reply.subarray(8,8+len)),pixels=reply.subarray(8+len);assert.equal(reply.readUInt32LE(4),pixels.length);assert.equal(metadata.status,'rendered');
    const browser=wasm.render_paths(json,component);assert.deepEqual(JSON.parse(browser.metadata),metadata);assert.deepEqual(Buffer.from(browser.take_pixels()),pixels);
    assert.ok(pixels.some((v,i)=>i%4===3&&v>0));
    const error=(upstream+4n)/5n+BigInt(metadata.info.work.coordinateErrorBound);assert.ok(error<=BigInt(request.viewport.coordinateTolerance));
    await fs.writeFile(path.join(path.dirname(output),item.name+'.rgba'),pixels,{flag:'wx'});
    record.raster={width:512,height:512,metadata,requestSha256:sha(json),pixelSha256:sha(pixels),pixelBytes:pixels.length,combinedCoordinateErrorPixelsQ32:String(error),exactNativeWasmPixels:true};
  }
  assert.equal(sha(await fs.readFile(item.source)),record.sourceSha256,'source must remain unchanged');
  records.push(record);
}
await fs.writeFile(output,JSON.stringify({profile:'source-circular-native-wasm-parity/1',cases:records,completeResponsesEqual:true,fullSourceChartRenderingProven:false,workbookRecalculationProven:false,officeWpsProven:false},null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({cases:records.length,compiled:records.filter(r=>r.response.status==='compiled').length,rendered:records.filter(r=>r.raster).length,completeResponsesEqual:true}));
