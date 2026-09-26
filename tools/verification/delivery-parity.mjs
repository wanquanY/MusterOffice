/** Actual public calls with the new writer and isolated transport. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/delivery-pipeline';
const candidate=process.argv[2];assert(candidate,'candidate diagnostic directory required');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const parent=JSON.parse(fs.readFileSync('docs/reviews/evidence/2026-09-26-job-results-verification.json'));
const require=createRequire(import.meta.url);
for(const key of ['rustWasm','rustWasmGlue'])assert.deepEqual(entry(parent.unchangedKernelArtifacts[key].path),parent.unchangedKernelArtifacts[key]);
const old=require('../../'+parent.unchangedKernelArtifacts.rustWasmGlue.path);
const wasm=require('../../'+root+'/wasm-node/mo_wasm.js');
const reports={};
for(const [name,script,args] of [
 ['authored','tools/verification/pptx-parity.mjs',[]],
 ['source','tools/verification/pptx-source-parity.mjs',['.codex-work/sealed-export/source-manifest.json']],
 ['editor','tools/verification/native-wasm-parity.mjs',[]],
]) {
 assert.deepEqual(entry(script),parent.sourceFiles.find(r=>r.path===script));
 const run=spawnSync(process.execPath,[script,'target/debug/mo-cli',root+'/wasm-node/mo_wasm.js',...args],{encoding:'utf8',timeout:120000,maxBuffer:64*1024*1024});
 fs.writeFileSync(`${root}/${name}-parity.stderr`,run.stderr,{flag:'wx'});
 fs.writeFileSync(`${root}/${name}-parity.json`,run.stdout,{flag:'wx'});
 assert.equal(run.status,0,run.stderr);assert.equal(run.stderr,'');
 reports[name]=JSON.parse(run.stdout);
 console.log(JSON.stringify({suite:name,cases:reports[name].passed}));
}
fs.mkdirSync(root+'/prior-authored');
let authored=0,rewrites=0,inspections=0,editor=0;
for(const c of reports.authored.cases) {
 const dir=reports.authored.artifactDirectory;
 const request=fs.readFileSync(`${dir}/${c.name}.json`,'utf8'),input=fs.readFileSync(`${dir}/${c.name}.bin`);
 if(c.status==='exported') {
   fs.writeFileSync(`${root}/prior-authored/${c.name}.pptx`,Buffer.from(old.export_pptx(request,input)),{flag:'wx'});authored++;
 } else assert.throws(()=>old.export_pptx(request,input),e=>String(e).startsWith(c.code+':'));
}
for(const c of reports.source.outputs) {
 const b=Buffer.from(old.edit_pptx_text(fs.readFileSync(c.request,'utf8'),fs.readFileSync(c.source)));
 assert.equal(sha(b),c.sha256);assert.deepEqual(b,fs.readFileSync(c.output));rewrites++;
}
for(const c of reports.source.cases)if(c.response){assert.deepEqual(JSON.parse(old.inspect_pptx(fs.readFileSync(c.source))),c.response);inspections++;}
for(const c of reports.editor.cases){assert.deepEqual(JSON.parse(old.dispatch_json(c.input)),c.response);editor++;}

const files=JSON.parse(fs.readFileSync(candidate+'/files.json'));
const file=name=>{const f=files.find(f=>f.name===name),b=fs.readFileSync(candidate+'/'+f.file);assert.equal(sha(b),f.asset.sha256);return b;};
const source=file('presentation'),font=file('font-bundle');
const context=JSON.parse(file('delivery-context')),settings=context.settings;
const skm=new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm'));
const hbm=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const pages=[];
for(let i=0;i<2;i++) {
 const evidence=JSON.parse(file(`preview-evidence:${i}`));
 const request={profile:'drawingml-resource-page-q32-v1-draft',page:{expectedSourceSha256:sha(source),slide:evidence.render.page.page.slide,profile:'drawingml-static-solid-page-v1-draft',colorContext:settings.colorContext,viewport:{width:640,height:360,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:19050},coordinateTolerance:'1048576',background:[0,0,0,0]}},imageSource:settings.imageSource,sampling:settings.sampling,fonts:settings.fonts};
 const j=Buffer.from(JSON.stringify(request)),header=Buffer.alloc(12);header.writeUInt32LE(j.length);header.writeUInt32LE(source.length,4);header.writeUInt32LE(font.length,8);
 const run=spawnSync('target/debug/mo-raster-worker',['--pptx-resource-page'],{input:Buffer.concat([header,j,source,font]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(run.status,0,run.stderr.toString());
 const n=run.stdout.readUInt32LE(),length=run.stdout.readUInt32LE(4);assert.equal(run.stdout.length,8+n+length);
 const metadata=run.stdout.subarray(8,8+n).toString(),pixels=run.stdout.subarray(8+n);
 const component=await RasterComponent.create(skiaFactory,skm),shaper=await ShapingComponent.create(hbFactory,hbm);
 const w=wasm.render_pptx_resource_page(j.toString(),source,font,component,shaper,component);
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);
 const response=JSON.parse(metadata);assert.equal(response.status,'rendered');assert.deepEqual(response.info,evidence.render);
 assert.equal(sha(pixels),evidence.render.page.scene.raster.sha256);
 component.invalidate();shaper.invalidate();
 const prefix=`${root}/page-${i}`;
 fs.writeFileSync(prefix+'.request.json',j,{flag:'wx'});fs.writeFileSync(prefix+'.response.json',metadata,{flag:'wx'});fs.writeFileSync(prefix+'.rgba',pixels,{flag:'wx'});
 pages.push({request:entry(prefix+'.request.json'),response:entry(prefix+'.response.json'),pixels:entry(prefix+'.rgba')});
}
const paths=['target/debug/mo-cli','target/debug/mo-raster-worker',root+'/wasm-node/mo_wasm.js',root+'/wasm-node/mo_wasm_bg.wasm','.codex-work/gradient-coordinates/component/mo-skia.mjs','.codex-work/gradient-coordinates/component/mo-skia.wasm','.codex-work/elliptic-source/ts-raster/index.js','.codex-work/text-component/index.js','.codex-work/harfbuzz/release/mo-hb.mjs','.codex-work/harfbuzz/release/mo-hb.wasm'];
const report={format:'musteroffice.delivery-parity/1',authoredCases:reports.authored.passed,sourceCases:reports.source.passed,editorCases:reports.editor.passed,previousAuthoredRegenerated:authored,previousEditedByteIdentical:rewrites,previousSourceResponsesIdentical:inspections,previousEditorResponsesIdentical:editor,pages,artifacts:paths.map(entry),reports:Object.fromEntries(Object.keys(reports).map(k=>[k,entry(`${root}/${k}-parity.json`)])),limitations:['Authored bytes intentionally change to matching-level defaults; independent ZIP/XML delta verification is separate.','Two actual delivery pages execute through both renderers; complete delivery computation has WASM compile evidence only.','No public export commit, full quality or target-application acceptance.']};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({authored,rewrites,inspections,editor,pages:pages.length}));
