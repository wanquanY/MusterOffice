// Compare complete public native/WASM chart responses for caller-owned inputs.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
const [cli, wasmModule, casesFile, output] = process.argv.slice(2);
assert.ok(output, 'usage: node pptx-chart-parity.mjs <cli> <wasm-node.js> <cases.json> <new-report.json>');
const cases=JSON.parse(await fs.readFile(casesFile,'utf8'));
const wasm=createRequire(import.meta.url)(path.resolve(wasmModule));
const records=[];
for(const item of cases){
  const request=await fs.readFile(item.request,'utf8'),source=await fs.readFile(item.source);
  const native=JSON.parse(execFileSync(cli,['pptx-charts',item.request,item.source],{encoding:'utf8',maxBuffer:32*1024*1024}));
  const browser=JSON.parse(wasm.inspect_pptx_charts(request,source));
  assert.deepEqual(browser,native);
  if(item.expectedStatus)assert.equal(native.status,item.expectedStatus);
  records.push({name:item.name,sourceSha256:createHash('sha256').update(source).digest('hex'),status:native.status,
    bindings:native.charts?.bindings.length??0,charts:native.charts?.charts.length??0,
    responseSha256:createHash('sha256').update(JSON.stringify(native)).digest('hex')});
}
const report={profile:'pptx-source-chart-native-wasm-parity/1',cases:records,
  renderingProven:false,workbookConsistencyProven:false,officeWpsProven:false};
await fs.writeFile(output,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({cases:records.length,completeResponsesEqual:true}));
