/** Real native/WASM pages and retained playback, using owned text-clipping fixtures.
 * Includes full metadata/pixel equality. It does not certify Office semantics.
 */
import assert from 'node:assert/strict';
import {readFileSync, readdirSync, mkdirSync, writeFileSync} from 'node:fs';
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {spawn, spawnSync} from 'node:child_process';

assert.equal(process.argv.length, 10, 'corpus worker wasm.js text.js raster.js harfbuzzDir skiaDir output');
const [corpus, worker, wasmPath, textPath, rasterPath, hbRoot, skiaRoot, output] = process.argv.slice(2).map(p=>resolve(p));
mkdirSync(output, {recursive:false});
const hash = b=>createHash('sha256').update(b).digest('hex');
const receipt = path=>{const bytes=readFileSync(path);return {path,sha256:hash(bytes),byteLength:bytes.length};};
const inputs = new Map();
function input(path) {const r=receipt(path);inputs.set(path,r);return readFileSync(path);}
function save(name,bytes) {const path=join(output,name);writeFileSync(path,bytes,{flag:'wx'});return receipt(path);}
function json(name,value) {return save(name,JSON.stringify(value,null,2)+'\n');}
for(const p of [worker,wasmPath,wasmPath.replace(/\.js$/,'_bg.wasm'),textPath,rasterPath,join(hbRoot,'mo-hb.mjs'),join(skiaRoot,'mo-skia.mjs')]) input(p);
const wasm=createRequire(import.meta.url)(wasmPath);
const {ShapingComponent}=await import(pathToFileURL(textPath));
const {RasterComponent}=await import(pathToFileURL(rasterPath));
const {default:hbFactory}=await import(pathToFileURL(join(hbRoot,'mo-hb.mjs')));
const {default:skiaFactory}=await import(pathToFileURL(join(skiaRoot,'mo-skia.mjs')));
const text=await ShapingComponent.create(hbFactory,new WebAssembly.Module(input(join(hbRoot,'mo-hb.wasm'))));
const raster=await RasterComponent.create(skiaFactory,new WebAssembly.Module(input(join(skiaRoot,'mo-skia.wasm'))));
const fonts=input(resolve('fixtures/fonts/owned-decorations.ttf'));
const manifest=JSON.parse(input(resolve('fixtures/fonts/decoration-manifest.json')));
const empty=Buffer.alloc(0);
function wire(q,source=empty,fontBytes=empty) {
  const bytes=Buffer.from(JSON.stringify(q)),h=Buffer.alloc(12);
  [bytes.length,source.length,fontBytes.length].forEach((n,i)=>h.writeUInt32LE(n,4*i));
  return Buffer.concat([h,bytes,source,fontBytes]);
}
function decode(bytes) {
  const ml=bytes.readUInt32LE(),pl=bytes.readUInt32LE(4);
  assert.equal(bytes.length,8+ml+pl);
  return {metadata:bytes.subarray(8,8+ml).toString(),pixels:bytes.subarray(8+ml)};
}
function native(mode,q,source) {
  const n=spawnSync(worker,[mode],{input:wire(q,source,fonts),timeout:60000,maxBuffer:80<<20});
  assert.ifError(n.error);assert.equal(n.status,0,n.stderr.toString());return decode(n.stdout);
}
function nativeOwner() {
  const child=spawn(worker,['--pptx-playback-session'],{stdio:['pipe','pipe','pipe']});
  let buffer=empty,pending=null,stderr='';
  child.stdout.on('data',b=>{
    buffer=Buffer.concat([buffer,b]);
    if(!pending||buffer.length<8)return;
    const length=8+buffer.readUInt32LE()+buffer.readUInt32LE(4);
    assert(length<=80<<20);
    if(buffer.length<length)return;
    const result=decode(buffer.subarray(0,length));buffer=buffer.subarray(length);
    const p=pending;pending=null;clearTimeout(p.timer);p.resolve(result);
  });
  child.stderr.on('data',b=>stderr+=b);
  const exit=new Promise((resolve,reject)=>{
    child.on('error',reject);
    child.on('close',code=>{if(pending){clearTimeout(pending.timer);pending.reject(new Error(stderr||'worker closed'));pending=null;}resolve(code);});
  });
  return {
    send:(q,source=empty,fontBytes=empty)=>new Promise((resolve,reject)=>{
      assert.equal(pending,null);pending={resolve,reject,timer:setTimeout(()=>{child.kill();reject(new Error('worker timeout'));},60000)};
      child.stdin.write(wire(q,source,fontBytes));
    }),
    close:async()=>{child.stdin.end();assert.equal(await exit,0,stderr);assert.equal(buffer.length,0);assert.equal(stderr,'');},
  };
}
function page(source,resource) {
  return {
    profile:resource?'drawingml-resource-page-q32-v1-draft':'drawingml-solid-text-page-q32-draft-v1',
    page:{expectedSourceSha256:hash(source),slide:'/ppt/slides/slide1.xml',profile:'drawingml-static-solid-page-v1-draft',colorContext:{systemColors:{},placeholder:null},viewport:{width:400,height:300,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:4000},coordinateTolerance:'16777216',background:[255,255,255,255]}},
    fonts:manifest,...(resource?{imageSource:'embeddedSnapshot',sampling:'nearest'}:{}),
  };
}
const staticCases=[],playbackCases=[];
try {
  for(const name of readdirSync(corpus).filter(n=>n.endsWith('.pptx')).sort()) {
    const source=input(join(corpus,name)),stem=name.slice(0,-5);
    const plan=JSON.parse(input(join(corpus,stem+'.json'))),expected=input(join(corpus,stem+'.rgba'));
    for(const mode of ['text','resource']) {
      const q=page(source,mode==='resource'),n=native(`--pptx-${mode}-page`,q,source);
      const w=mode==='text'?wasm.render_pptx_text_page(JSON.stringify(q),source,fonts,text,raster):wasm.render_pptx_resource_page(JSON.stringify(q),source,fonts,raster,text,raster);
      assert.equal(w.metadata,n.metadata,name);assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels,name);assert.deepEqual(n.pixels,expected,name+' Rust library pixels');
      const response=JSON.parse(n.metadata);assert.equal(response.status,'rendered',n.metadata);
      const capacity=response.info.textCapacity.frames;
      assert.equal(capacity.length,1);assert.equal(response.info.textFrames,1);
      assert.deepEqual(capacity[0].inkBounds,plan.texts[0].frame.bounds);
      assert.deepEqual(capacity[0].inner,plan.texts[0].frame.region.inner);
      assert.equal(capacity[0].contentHeight,plan.texts[0].frame.contentHeight);
      staticCases.push({name,mode,sourceSha256:hash(source),request:json(stem+'-'+mode+'.request.json',q),response:save(stem+'-'+mode+'.response.json',n.metadata),pixelSha256:hash(n.pixels),pixelBytes:n.pixels.length});
    }
  }
  assert.equal(staticCases.length,64);
  for(const name of readdirSync(join(corpus,'playback')).filter(n=>n.endsWith('.pptx')).sort()) {
    const stem=name.slice(0,-5),source=input(join(corpus,'playback',name)),binding={session:'clip-test',generation:'1',revision:hash(source)};
    const q={operation:'prepare',request:{page:page(source,true),binding}};
    const native=nativeOwner(),owner=new wasm.PptxPlaybackSession();
    try {
      const n=await native.send(q,source,fonts),metadata=owner.prepare(JSON.stringify(q),source,fonts,raster,text);
      assert.equal(metadata,n.metadata);assert.equal(JSON.parse(metadata).status,'prepared',metadata);
      const prepared=save(stem+'-prepare.response.json',metadata);
      for(const [ordinal,ms] of [0,250,500,750,1000,250].entries()) {
        const sample={binding,at:{ticks:String(ms),timescale:1000},history:null};
        const q={operation:'render',sample},n=await native.send(q),w=owner.render(JSON.stringify(q),raster);
        assert.equal(w.metadata,n.metadata);assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels);
        const expected=input(join(corpus,'playback',`${stem}-${ordinal}.rgba`));assert.deepEqual(n.pixels,expected);
        const response=JSON.parse(n.metadata);assert.equal(response.status,'rendered',n.metadata);
        assert.equal(response.info.page.textWork.componentCalls,0);
        const expectedState=JSON.parse(input(join(corpus,'playback',`${stem}-${ordinal}.state.json`)));
        assert.deepEqual(response.info.playback,expectedState);
        playbackCases.push({name,ordinal,sourceSha256:hash(source),prepared,request:json(`${stem}-${ordinal}.request.json`,q),response:save(`${stem}-${ordinal}.response.json`,n.metadata),pixelSha256:hash(n.pixels)});
      }
    } finally {owner.free();await native.close();}
  }
  assert.equal(playbackCases.length,90);
  for(const [path,before] of inputs)assert.deepEqual(receipt(path),before);
  json('report.json',{profile:'musteroffice.native-text-clipping-parity/1',status:'passed',staticCases,playbackCases,inputs:[...inputs.values()],scope:'Ordinary native text frames and shared clip pipeline; table-cell frames remain Rust library only.',officeAccepted:false});
  console.log(JSON.stringify({status:'passed',staticPairs:staticCases.length,retainedPairs:playbackCases.length,fullMetadataAndPixelsEqual:true}));
} finally {text.invalidate();raster.invalidate();}
