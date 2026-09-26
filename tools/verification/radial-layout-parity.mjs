/** Public native CLI and current WASM, no raster or shaping backend supplied. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
const root='.codex-work/radial-layout',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/radial-layout/wasm-node/mo_wasm.js');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const records=[];
const envelope=fills=>({profile:'drawingml-circle-path-bounds-q96-v1-draft',fills,options:{coordinateTolerance:'16777216'}});
function run(name,source,request,status,expected){
 const input=typeof request==='string'?request:JSON.stringify(request),q=put(out+'/'+name+'.request.json',input);
 const n=spawnSync('target/debug/mo-cli',['layout-pptx-radial',q.path,source],{env:{},timeout:60000,encoding:'utf8',maxBuffer:32*1024*1024});assert.equal(n.status,0,n.stderr);
 const native=n.stdout.trimEnd(),result=wasm.layout_pptx_radial(input,fs.readFileSync(source));assert.equal(result,native,name);
 const response=JSON.parse(result);assert.equal(response.status,status,name+': '+result);if(expected)assert.deepEqual(response.plans,expected,name);
 if(status==='error'){assert(!('plans' in response));assert(response.error.code);}
 records.push({name,source:entry(source),request:q,response:put(out+'/'+name+'.response.json',native),status,plans:response.plans?.length??0});
}
const files=fs.readdirSync(root+'/native').filter(f=>f.endsWith('.pptx')).sort();assert.equal(files.length,29);
for(const f of files){const name=f.slice(0,-5),base=root+'/native/'+name;run(name,base+'.pptx',envelope(JSON.parse(fs.readFileSync(base+'.query.json'))),'evaluated',JSON.parse(fs.readFileSync(base+'.plan.json')));}
const base=root+'/native/center-point',q=envelope(JSON.parse(fs.readFileSync(base+'.query.json')));
for(const [name,mutate] of [
 ['digest',q=>q.fills.expectedSourceSha256='0'.repeat(64)],['profile',q=>q.profile='unknown'],['unknown-field',q=>q.script='unused'],['duplicate-target',q=>q.fills.targets.push(q.fills.targets[0])],['missing-target',q=>q.fills.targets[0].nativeId=999999],['missing-surface',q=>q.fills.surface='/missing.xml'],['tolerance',q=>q.options.coordinateTolerance='256'],['noncanonical-fixed',q=>q.options.coordinateTolerance='01'],
]){const request=structuredClone(q);mutate(request);run('invalid-'+name,base+'.pptx',request,'error');}
run('invalid-duplicate-json',base+'.pptx','{"profile":0,"profile":0}','error');
for(const c of JSON.parse(fs.readFileSync(root+'/invalid.json')).cases){const request=structuredClone(q);request.fills.expectedSourceSha256=c.source.sha256;run('invalid-'+c.name,c.source.path,request,'error');}
const report={format:'musteroffice.radial-layout-parity/1',pairedCalls:records.length,evaluated:records.filter(c=>c.status==='evaluated').length,plans:records.reduce((a,c)=>a+c.plans,0),cases:records,artifacts:['target/debug/mo-cli',root+'/wasm-node/mo_wasm.js',root+'/wasm-node/mo_wasm_bg.wasm'].map(entry)};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({paired:report.pairedCalls,evaluated:report.evaluated,plans:report.plans,failed:report.pairedCalls-report.evaluated}));
