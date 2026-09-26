import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHarness} from './paragraph-harness.mjs';
const h=await createHarness('.codex-work/itemization');
const {root,fixture,paragraph,itemize,nativeParagraph,component,wasm}=h;
const simple=text=>({text,direction:'autoLeftToRight',spans:text?[{end:[...text].length,style:0}]:[]});
const representatives=JSON.parse(readFileSync(root+'/script-representatives.json'));
for(const [tag,cp] of Object.entries(representatives)) {const r=itemize('script-'+tag,simple(String.fromCodePoint(cp)));assert.equal(r.result.items[0].script,tag);}
const patterns=[['outer-pairs','A(γ)B',['Latn','Grek','Latn']],['outer-rtl','A(אב)B',['Latn','Hebr','Latn']],['kana','Aーア',['Latn','Kana']],['hira','あー',['Hira']],['shared-digit','\u{11800}\u0967',['Dogr']],['inherited-base','A\u05b0',['Latn']],['dotted-base','◌\u05b0',['Hebr']],['emoji-egc','👩🏽‍💻',['Zyyy']],['private-use','\ue000',['Zzzz']],['unknown','\u0378',['Zzzz']],['ambiguous','ー',['Hira']],['empty','',[]]];
for(const [name,text,scripts] of patterns)assert.deepEqual(itemize(name,simple(text)).result.items.map(i=>i.script),scripts);
const isolate=itemize('isolated-brackets',simple('A(\u2067אב(γ)\u2069)B'));assert.equal(isolate.result.items.filter(i=>i.kind==='bidiControl').length,2);
itemize('line-and-tab-controls',simple('A\tאב\u2028B\r\n'));
itemize('deep-brackets',simple('A'+'('.repeat(8000)+']'.repeat(8000)+'γ'+')'.repeat(8000)+'B'));
itemize('long-ascii',simple('A'.repeat(65536)));
itemize('no-coverage',{...simple('A'),spans:[]},'INPUT_INVALID');
itemize('split-egc',{...simple('A\u0301'),spans:[{end:1,style:0},{end:2,style:1}]},'INPUT_INVALID');
itemize('two-paragraphs',simple('A\nB'),'INPUT_INVALID');
itemize('too-many-scalars',simple('A'.repeat(65537)),'LIMIT_EXCEEDED');
itemize('item-limit',simple('Aγ'.repeat(2049)),'LIMIT_EXCEEDED');
itemize('bad-direction',JSON.stringify({...simple('A'),direction:'guess'}),'INPUT_INVALID',false);
for(const [name,text,names] of [
 ['latin-ligature','office',['notosans']],['arabic','العربية',['notosans','notosansarabic']],['indic','नमस्ते',['notosans','notosansdevanagari']],['mixed-four-scripts','office العربية नमस्ते 中文',['notosans','notosansarabic','notosansdevanagari','notosanssc']],
 ['paired-greek','A(γ)B',['notosans']],['arabic-latin-digits','A(العربية 123)B',['notosans','notosansarabic']],['jp-shared','Aーア',['notosans','notosanssc']],['arabic-isolate','A\u2067العربية\u2069 B',['notosans','notosansarabic']],
 ['join-controls','ب\u200cب ب\u200dب',['notosansarabic']],['combining','A\u0301',['notosans']],['supplementary','😀',['notoemoji']],['controls-only','\t\r\n',['owned']],['empty-shaping','',['owned']],['unavailable-item','A👩',['owned']],
]) {const r=paragraph(name,fixture(text,names));if(name==='unavailable-item')assert.ok(r.result.fallback.items[0].fragments.some(f=>f.status==='unresolved'));else assert.ok(r.result.fallback.items.every(i=>i.fragments.every(f=>f.status==='selected')),name);}
const equivalent=fixture('office');equivalent.request.styles.push(structuredClone(equivalent.request.styles[0]));equivalent.request.spans=[{end:2,style:0},{end:6,style:1}];
const merged=paragraph('equivalent-styles',equivalent);assert.equal(merged.result.fallback.items.length,1);assert.ok(merged.result.fallback.items[0].fragments[0].shaped.runs[0].glyphs.length<6);
const disabled=fixture('office');disabled.request.styles[0].features=[{tag:'liga',value:0,start:0,end:null}];assert.equal(paragraph('explicit-feature',disabled).result.fallback.items[0].fragments[0].shaped.runs[0].glyphs.length,6);
const different=fixture('AB');different.request.styles.push({...structuredClone(different.request.styles[0]),language:'fr'});different.request.spans=[{end:1,style:0},{end:2,style:1}];assert.equal(paragraph('different-languages',different).result.fallback.items.length,2);
const axis=fixture('office');axis.request.styles[0].candidates[0].variations=[{tag:'wght',value1616:(700<<16)+1}];paragraph('variable-axis',axis);
const base=fixture('A',['owned']);
for(const [name,mutate,expected] of [
 ['bad-style-ref',r=>r.spans[0].style=9,'INPUT_INVALID'],['styles-budget',r=>r.styles=Array(257).fill(r.styles[0]),'LIMIT_EXCEEDED'],['span-budget',r=>r.spans=Array(257).fill(r.spans[0]),'LIMIT_EXCEEDED'],
 ['candidate-budget',r=>r.styles[0].candidates=Array(33).fill(r.styles[0].candidates[0]),'LIMIT_EXCEEDED'],['axis-budget',r=>r.styles[0].candidates[0].variations=Array(65).fill({tag:'wght',value1616:0}),'LIMIT_EXCEEDED'],
 ['font-budget',r=>r.fonts=Array(33).fill(r.fonts[0]),'LIMIT_EXCEEDED'],['glyph-budget',r=>r.styles[0].maxGlyphs=0,'LIMIT_EXCEEDED'],
 ['unused-style',r=>r.styles.push({...structuredClone(r.styles[0]),language:'bad language'}),'INPUT_INVALID'],['unused-axis',r=>{const s=structuredClone(r.styles[0]);s.candidates[0].variations=[{tag:'xxxx',value1616:0}];r.styles.push(s);},'INPUT_INVALID'],
 ['empty-candidates',r=>r.styles[0].candidates=[],'INPUT_INVALID'],['wrong-font',r=>r.fonts[0].expectedSha256='0'.repeat(64),'RESOURCE_CONFLICT'],['bad-binding',r=>r.styles[0].candidates[0].font=1,'INPUT_INVALID'],
 ['bad-range',r=>r.fonts[0].offset='18446744073709551615','INPUT_INVALID'],['bad-feature',r=>r.styles[0].features=[{tag:'liga',value:0,start:2,end:3}],'INPUT_INVALID'],
]) {const f={bundle:base.bundle,request:structuredClone(base.request)};mutate(f.request);paragraph(name,f,expected);}
paragraph('duplicate-key',{bundle:base.bundle,request:JSON.stringify(base.request).replace('"text":"A"','"text":"A","text":"B"')},'INPUT_INVALID',false);
paragraph('unknown-field',{bundle:base.bundle,request:JSON.stringify({...base.request,path:'forbidden'})},'INPUT_INVALID',false);
const cli=fixture('office');writeFileSync(root+'/cli.json',JSON.stringify(cli.request));writeFileSync(root+'/cli-font.bin',cli.bundle);
const cp=spawnSync('target/release/mo-cli',['shape-paragraph',root+'/cli.json',root+'/cli-font.bin'],{encoding:'utf8',maxBuffer:20*1024*1024,timeout:30000});assert.equal(cp.status,0,cp.stderr);assert.equal(cp.stdout.trimEnd(),nativeParagraph(JSON.stringify(cli.request),cli.bundle));
// A broken component cannot return earlier selected items or trigger a font fallback.
let calls=0,invalid=false;
const failComponent={shapeBatch(font,frame){if(++calls===2)return Uint32Array.of(2,0);return component.shapeBatch(font,frame);},invalidate(){invalid=true;}};
const q=fixture('Aγ');const failed=JSON.parse(wasm.shape_paragraph(JSON.stringify(q.request),q.bundle,failComponent));assert.equal(failed.status,'error');assert.equal(failed.error.code,'COMPONENT_FAILURE');assert.equal(calls,2);assert.equal(invalid,true);assert.equal(failed.result,undefined);
h.finish({scriptRepresentatives:Object.keys(representatives).length,cliVerified:true,fatalComponentFailureAtomic:true});
