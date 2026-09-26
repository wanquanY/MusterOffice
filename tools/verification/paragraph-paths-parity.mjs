import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {createHarness} from './paragraph-harness.mjs';
const {root,fixture,wasm,component,sha,sources}=await createHarness('.codex-work/paragraph-paths');
const cases=[];let mappedGlyphs=0,mappedCommands=0,subEmuOrigins=0;
function native(json,bundle) {
 const q=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(q.length);head.writeUInt32LE(bundle.length,4);
 const r=spawnSync('target/release/mo-text-worker',['--paths'],{input:Buffer.concat([head,q,bundle]),maxBuffer:80*1024*1024,timeout:30000});
 assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
const U=1n<<32n;
function rounded(n,d){const sign=n<0n?-1n:1n;const a=n*sign;return sign*((a*2n+d)/(d*2n));}
function transformed(c,size,scale){const out={kind:c.kind};for(const [k,p] of Object.entries(c))if(k!=='kind')out[k]={x:String(rounded(BigInt(p.x)*size*U,scale)),y:String(-rounded(BigInt(p.y)*size*U,scale))};return out;}
function verifySources(q,bundle,result){
 const scene=result.scene;if(!scene)return;
 const sourcePaths=[];
 for(let i=0;i<scene.fonts.length;i++){
  const font=scene.fonts[i],binding=q.layout.paragraph.fonts.find(f=>f.expectedSha256===font.fontSha256&&f.faceIndex===font.faceIndex);assert.ok(binding);
  const bytes=bundle.subarray(Number(binding.offset),Number(binding.offset)+Number(binding.byteLength));
  const ids=[...new Set(scene.paths.filter(p=>p.font===i).map(p=>p.glyphId))];const lookup=new Map();
  for(let a=0;a<ids.length;a+=256){
   const request={expectedSha256:font.fontSha256,faceIndex:font.faceIndex,instances:[{variations:font.effectiveVariations.map(v=>({tag:v.tag,value1616:v.requested1616})),glyphIds:ids.slice(a,a+256),maxCommands:262144,maxOperations:1048576}]};
   const r=JSON.parse(wasm.outline_font(JSON.stringify(request),bytes,component));assert.equal(r.status,'outlined');
   for(const g of r.outlines.instances[0].glyphs)lookup.set(g.glyphId,g.path);
  }
  sourcePaths.push(lookup);
 }
 for(const p of scene.paths){
  const raw=sourcePaths[p.font].get(p.glyphId);assert.ok(raw);
  assert.deepEqual(p.commands,raw.map(c=>transformed(c,BigInt(p.fontSize),BigInt(scene.fonts[p.font].positionUnitsPerEm))));mappedCommands+=p.commands.length;
 }
 const geometry=result.layout.geometry;
 const positioned=geometry.layout.lines.flatMap((l,line)=>l.glyphs.map(g=>({...g,line})));
 assert.equal(scene.glyphs.length,positioned.length);
 for(let i=0;i<positioned.length;i++){
  const a=scene.glyphs[i],b=positioned[i];assert.equal(a.line,b.line);assert.equal(a.glyph,b.glyph);assert.deepEqual(a.source,b.source);
  for(const k of ['x','y']){assert.equal(rounded(BigInt(a.origin[k]),U),BigInt(b[k]));if(BigInt(a.origin[k])%U!==0n)subEmuOrigins++;}
  const f=geometry.shaping.fallback.items[a.source.fallbackItem].fragments[a.source.fragment];
  assert.equal(scene.paths[a.path].glyphId,f.shaped.runs[0].glyphs[a.glyph].glyphId);mappedGlyphs++;
 }
}
function run(name,q,bundle,expected='evaluated',validRequest=true){
 const json=typeof q==='string'?q:JSON.stringify(q),n=native(json,bundle),w=wasm.paragraph_paths(json,bundle,component);assert.equal(w,n,name);
 const response=JSON.parse(n);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 const requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`,bundlePath=`${root}/bundles/${sha(bundle)}.bin`;
 writeFileSync(requestPath,json);writeFileSync(responsePath,n);writeFileSync(bundlePath,bundle);
 cases.push({name,validRequest,requestPath,responsePath,bundlePath,requestSha256:sha(json),responseSha256:sha(n),bundleSha256:sha(bundle)});
 if(response.status==='evaluated'){
  const ordinary=JSON.parse(wasm.layout_paragraph(JSON.stringify(q.layout),bundle,component));assert.deepEqual(response.result.layout,ordinary.result);
  if(response.result.scene)assert.deepEqual(response.result.issues,[]);verifySources(q,bundle,response.result);
 }
 return response;
}
for(const c of JSON.parse(readFileSync('.codex-work/paragraph-layout/parity.json')).cases){
 const old=JSON.parse(readFileSync(c.responsePath)),q={layout:JSON.parse(readFileSync(c.requestPath)),boundsTolerance:String(1<<26)};
 run(c.name,q,readFileSync(c.bundlePath),old.status==='error'?old.error.code:'evaluated',c.validRequest);
}
function own(text,file,variation=false){
 const bytes=readFileSync(file);sources.custom=bytes;const f=fixture(text,['custom']);if(variation)f.request.styles[0].candidates[0].variations=[{tag:'wght',value1616:650*65536}];
 return {q:{layout:{paragraph:f.request,styles:[{fontSize:'254003',baselineShift:'-10001'}],strutStyle:0,spacing:{kind:'natural'},width:'800000',overflow:'keepUnbreakable'},boundsTolerance:String(1<<26)},bundle:f.bundle};
}
for(const [name,file,v] of [['quadratic-composite','fixtures/fonts/owned-outlines.ttf',false],['variable-composite','fixtures/fonts/owned-outlines.ttf',true],['cubic','fixtures/fonts/owned-outlines.otf',false],['no-outline',root+'/no-outline.ttf',false],['color-prerequisite',root+'/palette.ttf',false]]){
 const f=own('A B A',file,v);const r=run(name,f.q,f.bundle).result;
 if(name==='no-outline'){assert.equal(r.scene,null);assert.ok(r.issues.every(i=>i.kind==='outlineUnavailable'));}
 else if(name==='color-prerequisite'){assert.equal(r.scene,null);assert.equal(r.issues[0].kind,'colorRepresentationRequired');}
 else{assert.ok(r.scene.paths.length<r.scene.glyphs.length);}
}
const f=own('AAA','fixtures/fonts/owned-outlines.ttf');
for(const [name,value,valid=true] of [['zero-tolerance','0'],['below-tolerance','255'],['too-loose','4294967297'],['numeric-tolerance',256,false],['noncanonical-tolerance','0256',false],['overflow-tolerance','170141183460469231731687303715884105728',false]])run(name,{...f.q,boundsTolerance:value},f.bundle,'INPUT_INVALID',valid);
run('minimum-tolerance',{...f.q,boundsTolerance:'256'},f.bundle);
const alias=own('AAAA','fixtures/fonts/owned-outlines.ttf');alias.q.layout.paragraph.fonts.push(alias.q.layout.paragraph.fonts[0]);alias.q.layout.paragraph.styles.push(structuredClone(alias.q.layout.paragraph.styles[0]));alias.q.layout.paragraph.styles[1].candidates=[{font:1,variations:[]}];alias.q.layout.paragraph.spans=[{end:2,style:0},{end:4,style:1}];alias.q.layout.styles.push({fontSize:'127001',baselineShift:'5000'});
const a=run('font-alias-two-sizes',alias.q,alias.bundle).result.scene;assert.equal(a.fonts.length,1);assert.equal(a.work.uniqueSourceGlyphs,1);assert.equal(a.paths.length,2);
const json=JSON.stringify(f.q);writeFileSync(root+'/cli.json',json);writeFileSync(root+'/cli.bin',f.bundle);
const cli=spawnSync('target/release/mo-cli',['paragraph-paths',root+'/cli.json',root+'/cli.bin'],{encoding:'utf8',timeout:30000});assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trimEnd(),native(json,f.bundle));
// Fail the actual outline operation after layout and metrics have succeeded.
let raw,calls=0;const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default,faultBytes=readFileSync('.codex-work/harfbuzz/mo-hb.wasm');
const fault=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},new WebAssembly.Module(faultBytes));
const injecting={shapeBatch:(...args)=>fault.shapeBatch(...args),measureBatch:(...args)=>fault.measureBatch(...args),outlineBatch(font,frame){calls++;raw._mo_hb_fail_after(0);return fault.outlineBatch(font,frame);},invalidate:()=>fault.invalidate()};
const failure=JSON.parse(wasm.paragraph_paths(json,f.bundle,injecting));assert.equal(calls,1);assert.equal(failure.error.code,'COMPONENT_FAILURE');assert.equal(failure.result,undefined);assert.equal(fault.invalid,true);
const reuse=JSON.parse(wasm.paragraph_paths(json,f.bundle,injecting));assert.equal(reuse.error.code,'HOST_FAILURE');assert.equal(wasm.paragraph_paths(json,f.bundle,component),native(json,f.bundle));
const report={format:'musteroffice.paragraph-paths-parity/1',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),mappedGlyphs,mappedCommands,subEmuOrigins,cliVerified:true,actualPostLayoutFailure:{componentSha256:sha(faultBytes),failure,reuse,replacementVerified:true},cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,mappedGlyphs,mappedCommands,subEmuOrigins}));
