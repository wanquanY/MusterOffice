import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {parseArgs} from 'node:util';
const {values}=parseArgs({options:{directory:{type:'string',default:'.codex-work/harfbuzz'},'skip-faults':{type:'boolean',default:false}}});
const root=resolve(values.directory);
const create=(await import(pathToFileURL(root+'/mo-hb.mjs'))).default;
const binary=readFileSync(root+'/mo-hb.wasm'),compiled=new WebAssembly.Module(binary);
const hash=b=>createHash('sha256').update(b).digest('hex');
let forbiddenIoCalls=0;
async function fresh() {
  return create({instantiateWasm(imports,receive) {
    for (const name of ['fd_close','fd_write','fd_seek']) {
      if (imports.wasi_snapshot_preview1?.[name]) imports.wasi_snapshot_preview1[name]=()=>{
        forbiddenIoCalls++;throw new Error('Forbidden component I/O: '+name);
      };
    }
    const instance=new WebAssembly.Instance(compiled,imports);receive(instance,compiled);return instance.exports;
  }});
}
function wasmCall(m,c) {
  const font=readFileSync(c.font),request=readFileSync(c.request),lang=Buffer.from(c.language,'ascii');
  const allocations=[];
  function alloc(n){const p=m._malloc(Math.max(n,1));assert.ok(p,'host allocation failed');allocations.push(p);return p;}
  let out=0;
  try {
    const f=alloc(font.length),r=alloc(request.length),l=alloc(lang.length),slots=alloc(8);
    m.HEAPU8.set(font,f);m.HEAPU8.set(request,r);m.HEAPU8.set(lang,l);m.HEAPU32.fill(0,slots/4,slots/4+2);
    const status=m._mo_hb_shape(f,font.length,r,request.length/4,l,lang.length,slots,slots+4);
    out=m.HEAPU32[slots/4];const count=m.HEAPU32[slots/4+1];
    assert.equal(Boolean(out),count!==0);
    const words=out?Array.from(m.HEAPU32.subarray(out/4,out/4+count)):[];
    return {status,words};
  } finally {if(out)m._mo_hb_free(out);for(const p of allocations)m._free(p);}
}
function native(c,failAfter) {
  const args=[c.font,c.request,c.language];if(failAfter!==undefined)args.push(String(failAfter));
  const r=spawnSync(root+'/mo-hb-probe',args,{encoding:'utf8',maxBuffer:32*1024*1024});
  assert.equal(r.status,0,r.stderr);return JSON.parse(r.stdout);
}
function glyphs(words){assert.equal(words[0],0x4d4f4842);assert.equal(words.length,8+words[4]*7);
  return Array.from({length:words[4]},(_,i)=>{const p=8+i*7;return {g:words[p],cl:words[p+1],fl:words[p+2],ax:words[p+3]|0,ay:words[p+4]|0,dx:words[p+5]|0,dy:words[p+6]|0};});}
function reference(c) {
  const args=[c.font,'--text='+c.text,'--direction='+({4:'ltr',5:'rtl',6:'ttb',7:'btt'}[c.direction]),'--script='+c.script,'--language='+c.language,
    '--face-index='+c.face,'--font-size='+c.upem*64,'--font-funcs=ot','--shapers=ot','--cluster-level='+c.clusterLevel,'--no-glyph-names','--show-flags','-O','json'];
  for(const [bit,flag] of [[1,'--bot'],[2,'--eot'],[4,'--preserve-default-ignorables'],[8,'--remove-default-ignorables'],[64,'--unsafe-to-concat'],[128,'--safe-to-insert-tatweel']])if(c.flags&bit)args.push(flag);
  if(c.before)args.push('--text-before='+c.before);if(c.after)args.push('--text-after='+c.after);
  if(c.features.length)args.push('--features='+c.features.map(([tag,value,start,end])=>`${tag}[${Math.max(0,start-[...c.before].length)}:${end===0xffffffff?'':Math.max(0,end-[...c.before].length)}]=${value}`).join(','));
  if(c.variations.length)args.push('--variations='+c.variations.map(([tag,v])=>`${tag}=${v}`).join(','));
  const r=spawnSync(root+'/hb-shape-reference',args,{encoding:'utf8',maxBuffer:32*1024*1024});
  assert.equal(r.status,0,r.stderr);const result=JSON.parse(r.stdout);
  return result.map(g=>({...g,fl:g.fl??0,cl:g.cl+[...c.before].length}));
}
const manifest=JSON.parse(readFileSync('.codex-work/harfbuzz/cases/manifest.json'));
const m=await fresh();assert.equal(m._mo_hb_version(),0x0e0500);
assert.equal(typeof m._mo_hb_fail_after,values['skip-faults']?'undefined':'function');
const cases=[];
for(const c of manifest.cases) {
  assert.equal(hash(readFileSync(c.font)),c.fontSha256);assert.equal(hash(readFileSync(c.request)),c.requestSha256);
  const n=native(c),w=wasmCall(m,c);assert.equal(n.version,0x0e0500);assert.equal(n.status,c.expectedStatus,c.name);assert.deepEqual(w,{status:n.status,words:n.words},c.name);
  let count=0,compared=false;
  if(n.status===0) {
    const g=glyphs(n.words);count=g.length;
    if(c.text && !(c.flags&16)) {assert.deepEqual(g,reference(c),c.name+' differs from pinned upstream CLI');compared=true;}
    if(c.name==='deva-broken')assert.ok(g.some(x=>x.g===c.dottedCircleGlyph));
    if(c.name==='deva-no-circle')assert.ok(g.every(x=>x.g!==c.dottedCircleGlyph));
  } else assert.deepEqual(n.words,[]);
  cases.push({name:c.name,status:n.status,glyphs:count,referenceCompared:compared,referenceOmission:c.flags&16?"Upstream CLI exposes no dotted-circle suppression option; independently checked cmap glyph absence.":(!c.text?"Empty input has no upstream CLI line output.":null),fontSha256:c.fontSha256,requestSha256:c.requestSha256,resultSha256:hash(JSON.stringify(w))});
}
const c=manifest.cases[0],baseline=native(c),faults=[];
for(let i=0;i<(values['skip-faults']?0:300);i++) {
  const n=native(c,i);assert.ok([0,2].includes(n.status));
  if(n.status===2){assert.deepEqual(n.words,[]);assert.equal(n.recoveryStatus,6);assert.deepEqual(n.recoveryWords,[]);}
  else {assert.deepEqual(n.words,baseline.words);assert.deepEqual(n.recoveryWords,baseline.words);}
  const freshModule=await fresh();freshModule._mo_hb_fail_after(i);
  const w=wasmCall(freshModule,c);assert.ok([0,2].includes(w.status));
  freshModule._mo_hb_fail_after(0xffffffff);
  const reuse=wasmCall(freshModule,c);
  if(w.status===2){assert.deepEqual(w.words,[]);assert.equal(reuse.status,6);assert.deepEqual(reuse.words,[]);}
  else {assert.deepEqual(w.words,baseline.words);assert.deepEqual(reuse.words,baseline.words);}
  faults.push({failAfter:i,nativeStatus:n.status,wasmStatus:w.status,nativeReuseStatus:n.recoveryStatus,wasmReuseStatus:reuse.status});
  if(i%10===0)global.gc?.();
}
assert.equal(forbiddenIoCalls,0);
const result={format:'musteroffice.harfbuzz-component-verification/1',faultTests:!values['skip-faults'],nativeSha256:hash(readFileSync(root+'/mo-hb-probe')),wasmSha256:hash(binary),
  referenceSha256:hash(readFileSync(root+'/hb-shape-reference')),wasmImports:WebAssembly.Module.imports(compiled),forbiddenIoCalls,cases,faults};
writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({cases:cases.length,glyphs:cases.reduce((n,c)=>n+c.glyphs,0),referenceCases:cases.filter(c=>c.referenceCompared).length,
  faultPositions:faults.length,nativeFailures:faults.filter(f=>f.nativeStatus===2).length,wasmFailures:faults.filter(f=>f.wasmStatus===2).length,forbiddenIoCalls}));
