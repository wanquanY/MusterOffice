import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {createHarness} from './paragraph-harness.mjs';
import {verifyLineReference} from './line-reference.mjs';
const {root,fixture,wasm,component,sha,sources}=await createHarness('.codex-work/paragraph-layout');
const paths=new Map([[sha(sources.owned),'fixtures/fonts/owned.ttf']]);
for(const f of JSON.parse(readFileSync('fixtures/fonts/upstream.json')))if(f.name.endsWith('.ttf'))paths.set(f.sha256,`.codex-work/font-corpus/${f.family}/${f.name}`);
const cases=[];let referenceFragments=0,referenceGlyphs=0,referenceMetrics=0;
function native(json,bundle) {
 const q=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(q.length);head.writeUInt32LE(bundle.length,4);
 const r=spawnSync('target/release/mo-text-worker',['--layout'],{input:Buffer.concat([head,q,bundle]),maxBuffer:80*1024*1024,timeout:30000});
 assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
function input(text,names=['notosans'],width='800000',overflow='keepUnbreakable') {
 const f=fixture(text,names);return {bundle:f.bundle,request:{paragraph:f.request,styles:[{fontSize:'254000',baselineShift:'0'}],strutStyle:0,spacing:{kind:'natural'},width,overflow}};
}
function run(name,f,expected='evaluated',validRequest=true) {
 const json=JSON.stringify(f.request),n=native(json,f.bundle),w=wasm.layout_paragraph(json,f.bundle,component);assert.equal(w,n,name);
 const response=JSON.parse(n);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 const requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`,bundlePath=`${root}/bundles/${sha(f.bundle)}.bin`;
 writeFileSync(requestPath,json);writeFileSync(responsePath,n);writeFileSync(bundlePath,f.bundle);
 cases.push({name,kind:'paragraph-layout',validRequest,requestPath,responsePath,bundlePath,requestSha256:sha(json),responseSha256:sha(n),bundleSha256:sha(f.bundle)});
 if(response.status==='evaluated') {
  const r=response.result;
  assert.equal(r.geometry===null,r.issues.length>0);
  if(r.geometry) {
   assert.equal(r.decisions.length,r.geometry.shaping.lines.length);
   assert.equal(r.decisions.at(-1).end.scalarOffset,[...f.request.paragraph.text].length);
   const refs=verifyLineReference({paragraph:f.request.paragraph},r.geometry.shaping,paths);referenceFragments+=refs.referenceFragments;referenceGlyphs+=refs.referenceGlyphs;
   for(const m of r.geometry.metricInstances) {
    const font=f.request.paragraph.fonts[m.font];
    const ref=spawnSync('.codex-work/font-metrics/hb-metrics-reference',[paths.get(font.expectedSha256),String(font.faceIndex),'haschdschlgp',...m.variations.map(v=>`${v.tag}=${Math.fround(v.value1616/65536)}`)],{encoding:'utf8',timeout:30000});
    assert.equal(ref.status,0,ref.stderr);assert.deepEqual(m.measured.values.map(v=>v.position),JSON.parse(ref.stdout));referenceMetrics+=3;
   }
   let start=0;
   for(const d of r.decisions) {
    assert.ok(d.end.scalarOffset>=start);
    assert.ok(r.breaks.opportunities.filter(b=>b.kind==='mandatory').every(b=>b.boundary.scalarOffset<=start||b.boundary.scalarOffset>=d.end.scalarOffset));
    start=d.end.scalarOffset;
   }
  } else assert.equal(r.decisions.length,0);
 }
 return response;
}
for(const [name,text,names,width,overflow] of [
 ['latin','office AV office',['notosans'],'800000'],
 ['latin-wider','office AV office',['notosans'],'1600000'],
 ['arabic','بب 12 بب',['notosansarabic','notosans'],'500000'],
 ['arabic-emergency','بببب',['notosansarabic'],'250000','emergencyGrapheme'],
 ['bidi-isolate','A\u2067ب 12\u2069A',['notosans','notosansarabic'],'500000'],
 ['bidi-brackets','A (ب 12) A',['notosans','notosansarabic'],'500000'],
 ['devanagari','कि कि',['notosansdevanagari'],'200000','emergencyGrapheme'],
 ['cjk','中文演示文稿',['notosanssc'],'508000'],
 ['cjk-punctuation','中文，演示。',['notosanssc'],'508000'],
 ['emoji','A👩🏽‍💻A👩A',['owned','notoemoji'],'400000'],
 ['combining','A\u0301 α😀A',['owned'],'200000','emergencyGrapheme'],
 ['word-overflow','AAAAA',['owned'],'200000'],
 ['word-emergency','AAAAA',['owned'],'200000','emergencyGrapheme'],
 ['egc-overflow','A\u0301A',['owned'],'1','emergencyGrapheme'],
 ['space-mark-tailoring',' \u0301A',['owned'],'200000'],
 ['mandatory','A\u2028A',['owned'],'900000'],
 ['terminal-explicit','A\u2028',['owned'],'900000'],
 ['form-feed','A\fA\f',['owned'],'900000'],
 ['terminal-paragraph','A\r\n',['owned'],'900000'],
 ['empty','',['owned'],'1'],
 ['missing-font','A👩A',['owned'],'1'],
 ['tab','A\tA',['owned'],'500000'],
 ['soft-hyphen','A\u00adA',['owned'],'500000'],
 ['contingent-object','A\ufffcA',['owned'],'500000'],
]) run(name,input(text,names,width,overflow));
const tiny=input('AA',['owned'],'1','emergencyGrapheme');tiny.request.styles[0].fontSize='1';const r=run('sub-emu-fitting',tiny);assert.deepEqual(r.result.decisions.map(d=>d.end.scalarOffset),[1,2]);
const styled=input('office ب AV',['notosans','notosansarabic'],'700000','emergencyGrapheme');
const p=styled.request.paragraph;p.styles.push(structuredClone(p.styles[0]),structuredClone(p.styles[0]));p.spans=[{end:3,style:0},{end:8,style:1},{end:11,style:2}];styled.request.styles=[{fontSize:'254003',baselineShift:'10001'},{fontSize:'508001',baselineShift:'-20003'},{fontSize:'127001',baselineShift:'4001'}];run('mixed-styles',styled);
const features=input('office office',['notosans'],'700000','emergencyGrapheme');features.request.paragraph.styles[0].features=[{tag:'liga',value:0,start:2,end:11}];features.request.paragraph.styles[0].candidates[0].variations=[{tag:'wght',value1616:700*65536+1}];run('axes-and-global-features',features);
for(const [name,change,valid=true] of [
 ['zero-width',q=>q.width='0'],['negative-width',q=>q.width='-1'],['numeric-width',q=>q.width=100,false],['unknown-overflow',q=>q.overflow='truncate',false],['bad-style',q=>q.styles=[]],
 ['unused-bad-style',q=>{q.paragraph.styles.push({...structuredClone(q.paragraph.styles[0]),language:'bad language'});q.styles.push(q.styles[0]);}],
 ['multiple-paragraphs',q=>{q.paragraph.text='A\nA';q.paragraph.spans=[{end:3,style:0}];}],
]) {const f=input('AA',['owned']);change(f.request);run(name,f,'INPUT_INVALID',valid);}
run('cumulative-work-limit',input('A '.repeat(90),['owned'],'1'),'LIMIT_EXCEEDED');
run('maximum-empty-lines',input('\u2028'.repeat(4095),['owned'],'1'));
run('trailing-line-limit',input('\u2028'.repeat(4096),['owned'],'1'),'LIMIT_EXCEEDED');
const plain=input('A A A',['owned'],'1'),json=JSON.stringify(plain.request);writeFileSync(root+'/cli.json',json);writeFileSync(root+'/cli.bin',plain.bundle);
const cli=spawnSync('target/release/mo-cli',['layout-paragraph',root+'/cli.json',root+'/cli.bin'],{encoding:'utf8',timeout:30000});assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trimEnd(),native(json,plain.bundle));
let raw,calls=0;const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default,faultBytes=readFileSync('.codex-work/harfbuzz/mo-hb.wasm');
const fault=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},new WebAssembly.Module(faultBytes));
const injecting={shapeBatch(font,frame){if(++calls===2)raw._mo_hb_fail_after(0);return fault.shapeBatch(font,frame);},measureBatch(font,frame){return fault.measureBatch(font,frame);},invalidate(){fault.invalidate();}};
const failure=JSON.parse(wasm.layout_paragraph(json,plain.bundle,injecting));assert.equal(calls,2);assert.equal(failure.error.code,'COMPONENT_FAILURE');assert.equal(failure.result,undefined);assert.equal(fault.invalid,true);
const reuse=JSON.parse(wasm.layout_paragraph(json,plain.bundle,injecting));assert.equal(reuse.error.code,'HOST_FAILURE');assert.equal(wasm.layout_paragraph(json,plain.bundle,component),native(json,plain.bundle));
const report={format:'musteroffice.paragraph-layout-parity/1',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),
 referenceSha256:sha(readFileSync('.codex-work/harfbuzz/release/hb-shape-reference')),metricsReferenceSha256:sha(readFileSync('.codex-work/font-metrics/hb-metrics-reference')),referenceFragments,referenceGlyphs,referenceMetrics,cliVerified:true,actualSecondCandidateFailure:{componentSha256:sha(faultBytes),failure,reuse,replacementVerified:true},cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,referenceFragments,referenceGlyphs,referenceMetrics}));
