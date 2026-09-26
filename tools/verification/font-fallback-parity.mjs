import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {createHarness} from './paragraph-harness.mjs';
const h=await createHarness('.codex-work/font-fallback');
const {root,fixture,paragraph,component,wasm,sha,nativeParagraph}=h;
const selected=r=>r.result.fallback.items.flatMap(i=>i.fragments).filter(f=>f.status==='selected');
const single=r=>{assert.equal(r.result.fallback.items.length,1);return r.result.fallback.items[0];};
const basic=fixture('A👩A',['owned','notoemoji']);
const mixed=paragraph('latin-emoji-latin',basic);
assert.deepEqual(single(mixed).fragments.map(f=>[f.start,f.end,f.candidate]),[[0,1,0],[1,2,1],[2,3,0]]);
assert.equal(mixed.result.fallback.shapingRuns,5);assert.equal(mixed.result.fallback.componentCalls,4);
assert.notEqual(selected(mixed)[0].shaped.unitsPerEm,selected(mixed)[1].shaped.unitsPerEm);
for(const [name,text,names] of [
 ['retained-ligatures','office 👩 office',['notosans','notoemoji']],
 ['kerning-and-ligature','AV👩fi',['notosans','notoemoji']],
 ['arabic-emoji-arabic','ب👩ب',['notosansarabic','notoemoji']],
 ['arabic-connected-word','ببب',['owned','notosansarabic']],
 ['mixed-skin-tone-zwj','A👩🏽‍💻A',['owned','notoemoji']],
 ['family-zwj','A👨‍👩‍👧‍👦A',['owned','notoemoji']],
 ['regional-indicator-pair','A🇨🇳A',['owned','notoemoji']],
 ['combining-base','A\u0301👩A\u0301',['owned','notoemoji']],
 ['prefers-first-complete','office',['notosans','notoemoji']],
 ['prefer-primary-decomposition','Á👩A',['owned','notosans','notoemoji']],
]) {
 const r=paragraph(name,fixture(text,names));assert.ok(r.result.fallback.items.every(i=>i.fragments.every(f=>f.status==='selected')),name);
 if(name==='retained-ligatures') {assert.equal(single(r).fragments.length,3);assert.ok(single(r).protectedBoundaries.includes(2));assert.ok(single(r).protectedBoundaries.includes(3));}
 if(name==='kerning-and-ligature')assert.ok([1,4].every(n=>single(r).protectedBoundaries.includes(n)));
 if(name==='arabic-connected-word') {assert.deepEqual(single(r).protectedBoundaries,[1,2]);assert.equal(single(r).fragments[0].candidate,1);assert.equal(r.result.fallback.shapingRuns,2);}
 if(name==='mixed-skin-tone-zwj') {assert.deepEqual(single(r).fragments.map(f=>[f.start,f.end]),[[0,1],[1,5],[5,6]]);assert.equal(single(r).fragments[1].shaped.runs[0].glyphs.length,1);}
 if(name==='family-zwj') {assert.deepEqual(single(r).fragments.map(f=>[f.start,f.end]),[[0,1],[1,8],[8,9]]);assert.equal(single(r).fragments[1].shaped.runs[0].glyphs.length,1);}
 if(name==='regional-indicator-pair')assert.deepEqual(single(r).fragments.map(f=>[f.start,f.end]),[[0,1],[1,3],[3,4]]);
 if(name==='combining-base')assert.deepEqual(single(r).fragments.map(f=>[f.start,f.end]),[[0,2],[2,3],[3,5]]);
 if(name==='prefers-first-complete')assert.equal(r.result.fallback.shapingRuns,1);
 if(name==='prefer-primary-decomposition')assert.equal(single(r).fragments[0].candidate,0);
}
const noFont=paragraph('no-emoji-font',fixture('A👩A',['owned']));assert.deepEqual(single(noFont).fragments.map(f=>[f.start,f.end,f.status]),[[0,1,'selected'],[1,2,'unresolved'],[2,3,'selected']]);
const variation=paragraph('unsupported-selector',fixture('A\ufe02A',['owned']));assert.deepEqual(single(variation).fragments.map(f=>[f.start,f.end,f.status]),[[0,2,'unresolved'],[2,3,'selected']]);assert.equal(single(variation).probes[0].variationIssues.length,1);
const eq=fixture('office');eq.request.fonts.push(structuredClone(eq.request.fonts[0]));eq.request.styles[0].language='en-US';
eq.request.styles[0].candidates[0].variations=[{tag:'wght',value1616:700<<16},{tag:'wdth',value1616:95<<16}];
eq.request.styles.push(structuredClone(eq.request.styles[0]));eq.request.styles[1].language='EN-us';eq.request.styles[1].candidates[0].font=1;eq.request.styles[1].candidates[0].variations.reverse();eq.request.spans=[{end:2,style:0},{end:6,style:1}];
const before=JSON.stringify(eq.request),normalized=paragraph('equivalent-style-keys',eq);assert.equal(JSON.stringify(eq.request),before);assert.equal(normalized.result.itemization.items.length,1);assert.equal(selected(normalized)[0].shaped.runs[0].glyphs.length,4);assert.equal(normalized.result.fallback.verifiedFaces,1);
const duplicate=fixture('A');duplicate.request.styles.push(structuredClone(duplicate.request.styles[0]));duplicate.request.styles[1].candidates[0].variations=[{tag:'wght',value1616:400<<16},{tag:'wght',value1616:400<<16}];paragraph('normalized-invalid-axis-not-hidden',duplicate,'INPUT_INVALID');
paragraph('fragment-budget',fixture('A👩'.repeat(513),['owned','notoemoji']),'LIMIT_EXCEEDED');
paragraph('reshape-run-budget',fixture('A👩'.repeat(511)+'A',['owned','notoemoji']),'LIMIT_EXCEEDED');
const cli=fixture('A👩A',['owned','notoemoji']);writeFileSync(root+'/cli.json',JSON.stringify(cli.request));writeFileSync(root+'/cli-font.bin',cli.bundle);
const cp=spawnSync('target/release/mo-cli',['shape-paragraph',root+'/cli.json',root+'/cli-font.bin'],{encoding:'utf8',maxBuffer:20*1024*1024,timeout:30000});assert.equal(cp.status,0,cp.stderr);assert.equal(cp.stdout.trimEnd(),nativeParagraph(JSON.stringify(cli.request),cli.bundle));
// Actual component allocation failure after two valid full-item probes, exactly
// when mixed-font output would otherwise be reshaped. Never reuse the instance.
const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default;
const faultBytes=readFileSync('.codex-work/harfbuzz/mo-hb.wasm');let raw;
const fault=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},new WebAssembly.Module(faultBytes));
let calls=0;const injecting={shapeBatch(font,frame){if(++calls===3)raw._mo_hb_fail_after(0);return fault.shapeBatch(font,frame);},invalidate(){fault.invalidate();}};
const response=JSON.parse(wasm.shape_paragraph(JSON.stringify(basic.request),basic.bundle,injecting));assert.equal(response.error.code,'COMPONENT_FAILURE');assert.equal(calls,3);assert.equal(fault.invalid,true);assert.equal(response.result,undefined);
const reuse=JSON.parse(wasm.shape_paragraph(JSON.stringify(basic.request),basic.bundle,injecting));assert.equal(reuse.error.code,'HOST_FAILURE');
assert.equal(wasm.shape_paragraph(JSON.stringify(basic.request),basic.bundle,component),nativeParagraph(JSON.stringify(basic.request),basic.bundle));
h.finish({cliVerified:true,actualBoundaryReshapeAllocationFailure:{componentSha256:sha(faultBytes),response,reuseResponse:reuse,invalidated:true,replacementVerified:true},equivalentStylesVerified:true});
