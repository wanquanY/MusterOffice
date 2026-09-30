// Source-bound declared paints. Diagnostic frames deliberately contain only
// data-point fills; chart layout, stroke inheritance, labels and legends differ.
import fs from 'node:fs/promises';
import path from 'node:path';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const [configFile,output]=process.argv.slice(2);
assert.ok(output,'usage: node chart-paint-parity.mjs <config.json> <new-report.json>');
const config=JSON.parse(await fs.readFile(configFile,'utf8'));
const wasm=createRequire(import.meta.url)(path.resolve(config.wasmModule));
const sha=b=>createHash('sha256').update(b).digest('hex');
const {default:factory}=await import(pathToFileURL(path.resolve(config.componentModule)));
const {RasterComponent}=await import(pathToFileURL(path.resolve(config.adapterModule)));
const component=await RasterComponent.create(factory,new WebAssembly.Module(await fs.readFile(config.componentWasm)));
const records=[];
for(const item of config.cases){
  const request=await fs.readFile(item.request,'utf8'),source=await fs.readFile(item.source);
  const response=JSON.parse(execFileSync(config.cli,['pptx-chart-paints',item.request,item.source],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024}));
  assert.deepEqual(JSON.parse(wasm.compute_pptx_chart_paints(request,source)),response,item.name);
  assert.equal(response.status,item.expectedStatus,item.name);
  const record={name:item.name,requestSha256:sha(request),sourceSha256:sha(source),response};
  if(item.geometry){
    assert.equal(response.status,'computed');
    const raw=await fs.readFile(item.geometry,'utf8'),query=JSON.parse(raw);
    const geometry=JSON.parse(execFileSync(config.cli,['compile-pptx-chart-geometry',item.geometry,item.source],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024}));
    assert.deepEqual(JSON.parse(wasm.compile_pptx_chart_geometry(raw,source)),geometry);assert.equal(geometry.status,'compiled');
    assert.equal(geometry.geometry.chartSha256,response.paints.chart.sha256);
    assert.equal(query.outerRadius,String(1000n<<32n));assert.deepEqual(query.center,{x:'0',y:'0'});
    const paths=[],draws=[],fills=[];
    for(const series of geometry.geometry.series){
      const definition=response.paints.chart.plots.find(p=>p.sourceOrdinal===geometry.geometry.plotSourceOrdinal).series.find(s=>s.index===series.index);
      for(const curve of series.geometry.paths){
        if(!curve.commands.length)continue;
        const override=definition.pointOverrides.find(p=>p.index===curve.pointIndex);
        assert.ok(override,'diagnostic requires explicit point styles');
        const ordinal=override.layout.markup.find(m=>m.kind==='shapeProperties').sourceOrdinal;
        const declaration=response.paints.declarations.find(p=>p.sourceOrdinal===ordinal);
        assert.deepEqual(declaration.retainedOrdinals,[]);assert.equal(declaration.fill.definition.kind,'solid');assert.deepEqual(declaration.fill.retainedOrdinals,[]);
        assert.ok(!declaration.effects || (declaration.effects.definition.kind==='list' && !declaration.effects.definition.nodes.length && !declaration.effects.retainedOrdinals.length));
        const color=declaration.colors.find(c=>c.sourceOrdinal===declaration.fill.definition.color.sourceOrdinal);
        assert.equal(color.outcome.status,'resolved');
        draws.push({path:paths.length,origin:{x:'0',y:'0'},brush:{kind:'solid',rgba:color.outcome.rgba8}});paths.push({fillRule:series.geometry.fillRule,commands:curve.commands});
        fills.push({seriesIndex:series.index,pointIndex:curve.pointIndex,shapeOrdinal:ordinal,colorOrdinal:color.sourceOrdinal,rgba:color.outcome.rgba8});
      }
    }
    const scene={viewport:{width:512,height:512,origin:{x:String(-1280n<<32n),y:String(-1280n<<32n)},scale:{numerator:1,denominator:5},coordinateTolerance:String(1<<24),background:[0,0,0,0]},paths,draws};
    const json=JSON.stringify(scene),header=Buffer.alloc(4);header.writeUInt32LE(Buffer.byteLength(json));
    const wire=execFileSync(config.worker,[],{input:Buffer.concat([header,Buffer.from(json)]),timeout:30000,maxBuffer:16*1024*1024,env:{}});
    const n=wire.readUInt32LE(),metadata=JSON.parse(wire.subarray(8,8+n)),pixels=wire.subarray(8+n);assert.equal(wire.readUInt32LE(4),pixels.length);assert.equal(metadata.status,'rendered');
    const browser=wasm.render_paths(json,component);assert.deepEqual(JSON.parse(browser.metadata),metadata);assert.deepEqual(Buffer.from(browser.take_pixels()),pixels);assert.ok(pixels.some((v,i)=>i%4===3&&v));
    const upstream=geometry.geometry.series.reduce((a,s)=>a>BigInt(s.coordinateErrorBound)?a:BigInt(s.coordinateErrorBound),0n);
    const bound=(upstream+4n)/5n+BigInt(metadata.info.work.coordinateErrorBound);assert.ok(bound<=BigInt(scene.viewport.coordinateTolerance));
    await fs.writeFile(path.join(path.dirname(output),item.name+'.rgba'),pixels,{flag:'wx'});
    record.fillDiagnostic={fills,geometryRequestSha256:sha(raw),geometry,pixelSha256:sha(pixels),width:512,height:512,metadata,combinedCoordinateErrorPixelsQ32:String(bound),completePixelsAndMetadataEqual:true,strokeRendered:false,fullChartRendered:false};
  }
  assert.equal(sha(await fs.readFile(item.source)),record.sourceSha256);
  records.push(record);
}
await fs.writeFile(output,JSON.stringify({profile:'source-chart-declared-paint-parity/1',cases:records,completeResponsesEqual:true,fullChartStyleCascadeProven:false,fullTemplateRenderingProven:false,officeWpsProven:false},null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({cases:records.length,computed:records.filter(r=>r.response.status==='computed').length,fillDiagnosticFrames:records.filter(r=>r.fillDiagnostic).length,completeResponsesEqual:true}));
