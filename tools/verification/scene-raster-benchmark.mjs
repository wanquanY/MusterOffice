// Local, raster-only observation, not a presentation or performance gate.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {performance} from 'node:perf_hooks';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import {node,identity} from './scene-raster-fixtures.mjs';
import {point,U} from './path-raster-fixtures.mjs';
const root='.codex-work/scene-raster',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),componentBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm');
const sourcePath='.codex-work/paragraph-paths/latin.response.json',source=JSON.parse(fs.readFileSync(sourcePath)).result.scene;
const glyph=source.paths.find(p=>p.commands.length>3);assert(glyph);
const path={fillRule:'nonzero',commands:glyph.commands},count=1000;
const request={viewport:{width:1024,height:768,origin:point(),scale:{numerator:1,denominator:9525},coordinateTolerance:String(1<<24),background:[255,255,255,255]},scene:{paths:[path],transforms:[node(null,[String(U),String(U/8n),'0',String(U)])],instances:[]}};
for(let i=0;i<count;i++){
 const x=BigInt(20+(i%40)*22)*9525n*U,y=BigInt(40+Math.floor(i/40)*28)*9525n*U;
 request.scene.transforms.push(node(0,identity,point(x,y)));request.scene.instances.push({path:0,transform:i+1,brush:{kind:'solid',rgba:[28,42,60,255]}});
}
const duplicated=structuredClone(request);duplicated.scene.paths=Array.from({length:count},()=>structuredClone(path));duplicated.scene.instances.forEach((p,i)=>p.path=i);
const input={shared:JSON.stringify(request),duplicated:JSON.stringify(duplicated)},samples={shared:[],duplicated:[]},output={},modules={};
const module=new WebAssembly.Module(componentBytes);
for(const key of Object.keys(input))modules[key]=await RasterComponent.create(factory,module);
function run(key,measure){const before=performance.now(),result=wasm.render_scene(input[key],modules[key]),metadata=JSON.parse(result.metadata),pixels=result.take_pixels(),elapsed=performance.now()-before;
 assert.equal(metadata.status,'rendered');if(measure)samples[key].push(elapsed);
 if(!output[key])output[key]={metadata,sha256:sha(pixels)};else assert.equal(sha(pixels),output[key].sha256);
}
for(let i=0;i<3;i++)for(const key of Object.keys(input))run(key,false);
// Alternate order to avoid systematically assigning the first run to one case.
for(let i=0;i<12;i++)for(const key of i%2?['duplicated','shared']:['shared','duplicated'])run(key,true);
assert.equal(output.shared.sha256,output.duplicated.sha256);
assert.equal(output.shared.metadata.info.work.compiledPaths,1);assert.equal(output.duplicated.metadata.info.work.compiledPaths,count);
const median=a=>{const s=[...a].sort((a,b)=>a-b);return (s[5]+s[6])/2;};
const report={format:'musteroffice.scene-raster-benchmark/1',environment:{platform:os.platform(),release:os.release(),architecture:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,totalMemory:os.totalmem(),node:process.version},rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),sourcePath,sourceSha256:sha(fs.readFileSync(sourcePath)),fontInstances:source.fonts,instances:count,sourceCommandsPerGlyph:path.commands.length,warmupIterationsPerCase:3,measuredIterationsPerCase:12,order:'alternating shared/duplicated after warmups',samplesMilliseconds:samples,medianMilliseconds:Object.fromEntries(Object.entries(samples).map(([k,v])=>[k,median(v)])),inputByteLengths:Object.fromEntries(Object.entries(input).map(([k,v])=>[k,Buffer.byteLength(v)])),outputs:output,scope:'Precomputed owned fixture text outlines only. Timed JSON parse, affine lowering/certification, Skia CPU drawing, response serialization and pixel copying; SHA assertion is outside timing. Fonts/shaping/layout, startup, file publication, peak RSS, GPU, target apps and full product budgets excluded. No GC forcing or cold-cache claim.'};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({medians:report.medianMilliseconds,inputBytes:report.inputByteLengths,compiledPaths:{shared:1,duplicated:count},pixelsEqual:true}));
