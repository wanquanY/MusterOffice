// Real Native/WASM paragraph interaction, renderer agreement and thin facade.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
import {createHash} from 'node:crypto';
const root=path.resolve(process.argv[2]??'.codex-work/text-interaction');
const componentRoot=path.resolve(process.env.MO_INTERACTION_COMPONENT_ROOT??'.codex-work/caret');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const wasm=createRequire(import.meta.url)(path.join(root,'wasm-node/mo_wasm.js'));
const {ShapingComponent}=await import(pathToFileURL(path.join(componentRoot,'text-component/index.js')));
const {default:factory}=await import(pathToFileURL(path.join(componentRoot,'harfbuzz/mo-hb.mjs')));
const hbBytes=readFileSync(path.join(componentRoot,'harfbuzz/mo-hb.wasm'));
const component=await ShapingComponent.create(factory,new WebAssembly.Module(hbBytes));
const {PresentationEditor,EditorComputationError}=await import(pathToFileURL(path.resolve('.codex-work/editor-client/build/editor-client/src/index.js')));
const editor=new PresentationEditor(wasm),cases=[];
const output=path.join(root,'parity');mkdirSync(output,{recursive:true});
const fixed=n=>(BigInt(n)*(1n<<32n)).toString();
const position=(scalarOffset,affinity='downstream')=>({scalarOffset,affinity});
const caret=(offset,affinity)=>({kind:'caret',position:position(offset,affinity)});
const hit=(x,y)=>({kind:'hit',point:{x:fixed(x),y:fixed(y)}});
const selection=(a,b)=>({kind:'selection',anchor:position(a),focus:position(b,'upstream')});
function query(text,bytes) {
 return {layout:{paragraph:{text,direction:'leftToRight',spans:text?[{end:[...text].length,style:0}]:[],
  styles:[{language:'und',features:[],candidates:[{font:0,variations:[]}],suppressDottedCircle:false,maxGlyphs:4096}],
  fonts:[{expectedSha256:sha(bytes),faceIndex:0,offset:'0',byteLength:String(bytes.length)}]},
  styles:[{fontSize:'1000',baselineShift:'0'}],strutStyle:0,spacing:{kind:'natural'},width:'10000',overflow:'keepUnbreakable'},
  queries:[caret(0),caret([...text].length,'upstream'),selection(0,[...text].length),hit(0,500),hit(-100,-100)]};
}
function native(json,bytes,mode='--interaction') {
 const request=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(request.length);head.writeUInt32LE(bytes.length,4);
 const r=spawnSync('target/release/mo-text-worker',[mode],{input:Buffer.concat([head,request,bytes]),maxBuffer:64*1024*1024,timeout:30000});
 assert.equal(r.error,undefined);assert.equal(r.status,0,r.stderr?.toString());
 assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
function run(label,q,bytes,expected='evaluated',mapExpected=true) {
 const json=typeof q==='string'?q:JSON.stringify(q);
 const n=native(json,bytes),w=wasm.paragraph_interaction(json,bytes,component);
 assert.equal(w,n,label+' native/wasm');const result=JSON.parse(n);
 assert.equal(result.status==='error'?result.error.code:result.status,expected,result.status==='error'?label+': '+n:label);
 if(result.status==='evaluated') {
  const api=editor.paragraphInteraction(q,bytes,component);assert.deepEqual(api,result.result,label+' facade');
  assert.equal(api.map!==null,mapExpected,label+' availability');
  const paths=JSON.parse(native(JSON.stringify({layout:q.layout,boundsTolerance:'256'}),bytes,'--paths'));
  assert.equal(paths.status,'evaluated',JSON.stringify(paths));
  assert.deepEqual(api.layout,paths.result.layout,label+' rendered layout');
  if(api.map) {
   assert.equal(api.map.cells.length+1,api.map.boundaries.length);
   api.map.cells.forEach((cell,i)=>{assert.deepEqual(cell.start,api.map.boundaries[i]);assert.deepEqual(cell.end,api.map.boundaries[i+1]);assert.ok(api.map.lines[cell.line].cells.includes(i));});
   assert.equal(api.results.length,q.queries.length);
  } else assert.deepEqual(api.results,[]);
 }
 writeFileSync(path.join(output,label+'.request.json'),json);writeFileSync(path.join(output,label+'.response.json'),n);
 cases.push({label,fontSha256:sha(bytes),requestSha256:sha(json),responseSha256:sha(n),status:expected,mapExpected});return result;
}
const owned=readFileSync('fixtures/fonts/owned-interaction.ttf');
for(const [name,text] of [
 ['ligature','AAA AAA'],['bidi','AאבA'],['combining','A\u0301 😀'],['empty',''],
 ['soft-end','A\u2028'],['paragraph-end','A\n'],['crlf','A\r\n'],['soft-wrap','A A A'],
 ['bidi-controls','A\u202bאב\u202cA'],['controls-only','\u202b\u202c'],['spaces','   '],
 ['rtl','אבג'],['isolates','A\u2067אב\u2069A'],['zh-alias','中文中文'],
 ]) {
 const q=query(text,owned);if(name==='rtl')q.layout.paragraph.direction='rightToLeft';
 if(name==='soft-wrap')q.layout.width='1200';
 if(name==='ligature')q.queries=[0,1,2,3].map(i=>caret(i,'upstream'));
 if(name==='bidi')q.queries=[caret(1,'upstream'),caret(1),caret(3,'upstream'),caret(3),hit(650,500)];
 const r=run(name,q,owned).result;
 if(name==='ligature')assert.deepEqual(r.results.map(v=>v.caret.edge.x),[0,210,468,577].map(fixed));
 if(name==='bidi') {assert.deepEqual(r.results.slice(0,4).map(v=>v.caret.edge.x),[600,1800,600,1800].map(fixed));assert.deepEqual(r.results[4].caret.position,position(3,'upstream'));}
}
for(const tracking of [100,-100,-700]) {
 const q=query('AA',owned);q.layout.styles[0].clusterSpacing=fixed(tracking);
 run('tracking-'+tracking,q,owned);
}
for(const baseline of [125,-125]) {
 const q=query('AAA',owned);q.layout.styles[0].baselineShift=String(baseline);
 run('baseline-'+baseline,q,owned);
}
for(const height of [0,500]) {
 const q=query('A\u2028A',owned);q.layout.spacing={kind:'styleMaximum',heights:[fixed(height)]};
 run('line-height-'+height,q,owned);
}
const mixed=query('A A',owned);mixed.layout.styles.push({fontSize:'750',baselineShift:'125'});
mixed.layout.paragraph.styles.push(structuredClone(mixed.layout.paragraph.styles[0]));mixed.layout.paragraph.spans=[{end:2,style:0},{end:3,style:1}];
run('mixed-styles',mixed,owned);
const tracking=readFileSync('fixtures/fonts/owned-tracking.ttf');
const absent=run('absent-carets',query('AA',tracking),tracking).result;
assert.deepEqual(absent.map.cells[0].placement,{kind:'clusterPartition',reason:'fontCaretsAbsent'});
for(const text of ['A\tA','A\u00adA','A\ufffcA','unknown Ω'])run('prerequisite-'+text.codePointAt(1),query(text,owned),owned,'evaluated',false);
const pinned=JSON.parse(readFileSync('fixtures/fonts/upstream.json'));
for(const [family,text,direction] of [
 ['notosans','office ffi cafe\u0301','leftToRight'],
 ['notosansarabic','السَّلَامُ عَلَيْكُمْ لا','rightToLeft'],
 ['notosansdevanagari','नमस्ते क्षि हिन्दी','leftToRight'],
 ['notosanssc','中文编辑，测试换行。','leftToRight'],
 ['notoemoji','👩🏽‍💻😀👨‍👩‍👧‍👦','leftToRight'],
]) {
 const pin=pinned.find(p=>p.family===family&&p.name.endsWith('.ttf'));
 const bytes=readFileSync(path.join('.codex-work/font-corpus',family,pin.name));assert.equal(sha(bytes),pin.sha256);
 for(const width of [10000,2000]) {
  const q=query(text,bytes);q.layout.paragraph.direction=direction;q.layout.width=String(width);
  run(family+'-'+width,q,bytes);
 }
}
for(const [name,change,code] of [
 ['inside-grapheme',q=>q.queries.push(caret(1)),'INPUT_INVALID'],
 ['past-end',q=>q.queries.push(caret(99)),'INPUT_INVALID'],
 ['query-limit',q=>q.queries=Array(65).fill(caret(0)),'LIMIT_EXCEEDED'],
 ['unknown-field',q=>q.clientGeometry={},'INPUT_INVALID'],
 ['font-digest',q=>q.layout.paragraph.fonts[0].expectedSha256='0'.repeat(64),'RESOURCE_CONFLICT'],
]) {
 const q=query('A\u0301',owned);change(q);run(name,q,owned,code);
 assert.throws(()=>editor.paragraphInteraction(q,owned,component),e=>e instanceof EditorComputationError&&e.diagnostic.code===code);
}
const valid=query('A\u0301',owned),json=JSON.stringify(valid);
run('duplicate-key',json.replace('"queries":','"queries":[],"queries":'),owned,'INPUT_INVALID');
let invoked=false;
const noCalls=new Proxy({}, {get(){return ()=>{invoked=true;throw Error('must preflight');};}});
valid.queries.push(caret(1));assert.throws(()=>editor.paragraphInteraction(valid,owned,noCalls));assert.equal(invoked,false);
const report={format:'musteroffice.text-interaction-parity/1',nativeWasmByteEquality:true,rendererLayoutEquality:true,facadeEquality:true,
 fontOracle:'owned GDEF 173/431 + GPOS (37,11), advance 577; synthetic aliases are not language-quality claims',
 nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync(path.join(root,'wasm-node/mo_wasm_bg.wasm'))),harfbuzzWasmSha256:sha(hbBytes),cases};
writeFileSync(path.join(root,'parity.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({cases:cases.length,evaluated:cases.filter(c=>c.status==='evaluated').length}));
