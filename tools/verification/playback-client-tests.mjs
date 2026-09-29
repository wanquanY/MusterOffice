/** Failure/ownership tests for the JS boundary, not a replacement renderer. */
import assert from 'node:assert/strict';
import test from 'node:test';
import {WasmPlayback, PlaybackComputationError, PlaybackStateError} from '../../.codex-work/playback-client/build/playback-client/src/index.js';
import {fitsUtf8} from '../../.codex-work/playback-client/build/playback-client/src/owner.js';

const binding={session:'test',revision:'a'.repeat(64),generation:'9007199254740993'};
const authorRequest=()=>({binding:{...binding},snapshot:{semanticDigest:'b'.repeat(64)},slide:'one',viewport:{width:1,height:1}});
const sourceRequest=()=>({binding:{...binding},page:{page:{slide:'/ppt/slides/slide1.xml',expectedSourceSha256:'c'.repeat(64),viewport:{width:1,height:1}}}});
const failure={status:'error',error:{kind:'session',code:'GENERATION_NOT_INCREASING',message:'invalid'}};
const ports={source:new Uint8Array(1),fonts:new Uint8Array(1),decoder:{},shaping:{}};
function fixture(kind='author') {
  const observed={free:0,frameFree:0,take:0,commands:[],callback:null,badFrame:null,commandOverride:null,freeError:null};
  const info={binding:{...binding},profile:'test',planId:'plan',slide:kind==='author'?'one':'/ppt/slides/slide1.xml',
    ...(kind==='author'?{documentSha256:'b'.repeat(64)}:{sourceSha256:'c'.repeat(64),preparation:{decodedImages:1}})};
  class Raw {
    command(text) {
      const q=JSON.parse(text);observed.commands.push(q);
      if(observed.commandOverride) return observed.commandOverride(q);
      if(q.operation==='prepare')return JSON.stringify({status:'prepared',info});
      if(q.operation==='advance'){
        if(BigInt(q.generation)<=BigInt(info.binding.generation))return JSON.stringify(failure);
        info.binding.generation=q.generation;return JSON.stringify({status:'advanced',info});
      }
      if(q.operation==='inspectTiming')return JSON.stringify({status:'timingInspected',info:{binding:info.binding,sampler:{}}});
      if(q.operation==='dispose')return JSON.stringify({status:'disposed',binding:info.binding});
      throw Error('unexpected command');
    }
    prepare(text){return this.command(text);}
    prepare_render(text){
      observed.pendingCreated=(observed.pendingCreated??0)+1;
      return {request:text,failure:observed.prepareFailure??'',begin:raster=>raster.beginRaster(new Uint32Array(4)),
        begin_validation(reply){
          observed.pendingConsumed=(observed.pendingConsumed??0)+1;
          if(observed.validationBeginError)throw observed.validationBeginError;
          let steps=0;
          return {request:text,reply,get failed(){return observed.validationFailed??false;},step(){observed.validationCallback?.();if(observed.validationStepError)throw observed.validationStepError;return ++steps>=2;},
            free(){observed.validationFreed=(observed.validationFreed??0)+1;if(observed.validationFreeError)throw observed.validationFreeError;}};
        },
        free(){observed.pendingFreed=(observed.pendingFreed??0)+1;if(observed.pendingFreeError)throw observed.pendingFreeError;}};
    }
    complete_render(pending,reply){
      observed.pendingConsumed=(observed.pendingConsumed??0)+1;
      if(reply.status!==0)return {metadata:JSON.stringify(failure),invalidates_backend:reply.status===2||reply.status===4,
        take_pixels:()=>new Uint8Array(0),free:()=>observed.frameFree++};
      return Object.assign(this.render(pending.request),{invalidates_backend:observed.invalidates??false});
    }
    complete_validation(validation){
      observed.validationConsumed=(observed.validationConsumed??0)+1;
      if(observed.validationCompleteError)throw observed.validationCompleteError;
      if(observed.validationFailed)return {metadata:JSON.stringify(failure),invalidates_backend:true,
        take_pixels:()=>new Uint8Array(0),free:()=>observed.frameFree++};
      return Object.assign(this.render(validation.request),{invalidates_backend:observed.invalidates??false});
    }
    render(text){
      const q=JSON.parse(text);observed.commands.push(q);observed.callback?.();
      if(observed.badFrame)return observed.badFrame;
      const state={binding:q.sample.binding,time:{ticks:'0',timescale:1}};
      const page={scene:{raster:{width:1,height:1,byteLength:'4'}}};
      const result=kind==='author'?{frame:{state},page}:{playback:{evaluated:{state},sourceSha256:info.sourceSha256,slide:info.slide},page:{page}};
      const pixels=Uint8Array.of(4,3,2,5);observed.pixels=pixels;
      return {metadata:JSON.stringify({status:'rendered',info:result}),take_pixels(){observed.take++;return pixels;},free(){observed.frameFree++;}};
    }
    free(){observed.free++;if(observed.freeError)throw observed.freeError;}
  }
  const sdk=new WasmPlayback({PlaybackSession:Raw,PptxPlaybackSession:Raw});
  return {sdk,observed,open:()=>kind==='author'?sdk.prepareAuthor(authorRequest()):sdk.prepareSource(sourceRequest(),ports)};
}
for(const kind of ['author','source']) {
  test(`${kind}: exact generations, detached metadata and single-consume frame ownership`,()=>{
    const {open,observed}=fixture(kind);const owner=open();
    const copy=owner.info;copy.binding.generation='1';copy.planId='changed';
    assert.equal(owner.info.binding.generation,binding.generation);assert.equal(owner.info.planId,'plan');
    const frame=owner.sample({ticks:'0',timescale:4},{});
    assert.equal(frame.pixels,observed.pixels);assert.equal(observed.take,1);assert.equal(observed.frameFree,0);
    assert.throws(()=>owner.advance('0'),PlaybackComputationError);assert.equal(owner.closed,false);
    assert.equal(owner.advance('9007199254740994').binding.generation,'9007199254740994');
    assert.equal(owner.timing().binding.generation,'9007199254740994');
    owner.dispose();assert.equal(owner.closed,true);owner.close();assert.equal(observed.free,1);
    assert.throws(()=>owner.timing(),e=>e instanceof PlaybackStateError&&e.code==='CLOSED');
  });
  test(`${kind}: prepare rejection releases allocation and retains core diagnostic`,()=>{
    const f=fixture(kind);f.observed.commandOverride=()=>JSON.stringify(failure);
    assert.throws(f.open,e=>e instanceof PlaybackComputationError&&e.kind===kind&&e.diagnostic.code==='GENERATION_NOT_INCREASING');
    assert.equal(f.observed.free,1);
  });
  test(`${kind}: a component callback cannot reenter or free an in-use owner`,()=>{
    const f=fixture(kind),owner=f.open();let callbacks=0;
    f.observed.callback=()=>{
      callbacks++;
      for(const action of [()=>owner.close(),()=>owner.dispose(),()=>owner.timing()])
        assert.throws(action,e=>e instanceof PlaybackStateError&&e.code==='BUSY');
      assert.equal(f.observed.free,0);
    };
    owner.sample({ticks:'0',timescale:1},{});assert.equal(callbacks,1);owner.dispose();assert.equal(f.observed.free,1);
  });
  test(`${kind}: malformed disposal still releases; cleanup failure is never hidden`,()=>{
    const f=fixture(kind),owner=f.open();f.observed.commandOverride=()=>'{';f.observed.freeError=Error('release failed');
    assert.throws(()=>owner.dispose(),e=>e instanceof AggregateError&&e.errors[0] instanceof SyntaxError&&e.errors[1]===f.observed.freeError);
    assert.equal(owner.closed,true);owner.close();assert.equal(f.observed.free,1);
  });
}
test('frame metadata failure frees unconsumed allocation; failed take is never freed twice',()=>{
  for(const mode of ['parse','getter','take','free']){
    const f=fixture(),owner=f.open();let freed=0,taken=0;
    f.observed.badFrame={get metadata(){if(mode==='getter')throw Error('metadata');return mode==='take'?'{}':'{';},
      take_pixels(){taken++;throw Error('take');},free(){freed++;if(mode==='free')throw Error('frame release');}};
    assert.throws(()=>owner.sample({ticks:'0',timescale:1},{}),mode==='free'?AggregateError:Error);
    assert.equal(taken,mode==='take'?1:0);assert.equal(freed,mode==='take'?0:1);
    assert.equal(f.observed.free,1);assert.equal(owner.closed,true);
  }
});
test('semantic frame errors preserve owner, but pixels on an error invalidate it',()=>{
  for(const bytes of [0,4]){
    const f=fixture(),owner=f.open();let consumed=0;
    f.observed.badFrame={metadata:JSON.stringify(failure),take_pixels(){consumed++;return new Uint8Array(bytes);},free(){assert.fail('already consumed');}};
    assert.throws(()=>owner.sample({ticks:'0',timescale:1},{}),bytes?PlaybackStateError:PlaybackComputationError);
    assert.equal(owner.closed,!!bytes);assert.equal(consumed,1);owner.close();assert.equal(f.observed.free,1);
  }
});
test('serialization failure and shared source buffers cannot leak owners',()=>{
  const f=fixture();const cyclic=authorRequest();cyclic.snapshot.self=cyclic;
  assert.throws(()=>f.sdk.prepareAuthor(cyclic),TypeError);assert.equal(f.observed.free,1);
  const source=fixture('source');assert.throws(()=>source.sdk.prepareSource(sourceRequest(),{...ports,source:new Uint8Array(new SharedArrayBuffer(1))}),PlaybackStateError);
  assert.equal(source.observed.commands.length,0);assert.equal(source.observed.free,1);
});
test('input accessors are borrowed once, and UTF-8 limits count bytes across surrogate cases',()=>{
  const f=fixture('source');let sourceReads=0,fontReads=0;
  const owner=f.sdk.prepareSource(sourceRequest(),{...ports,
    get source(){sourceReads++;return ports.source;},get fonts(){fontReads++;return ports.fonts;}});
  assert.equal(sourceReads,1);assert.equal(fontReads,1);owner.close();
  for(const text of ['ascii','办公内核','🙂x','\ud800','x\udc00','\ud800\udc00','']){
    const bytes=Buffer.byteLength(text);
    for(const limit of [0,Math.max(0,bytes-1),bytes,bytes+1])assert.equal(fitsUtf8(text,limit),bytes<=limit);
  }
});
test('wrong output binding/time and dimension/byte length invalidate the owner',()=>{
  for(const fault of ['binding','time','width','bytes']){
    const f=fixture(),owner=f.open();
    const info={frame:{state:{binding:{...binding},time:{ticks:'0',timescale:1}}},page:{scene:{raster:{width:1,height:1,byteLength:'4'}}}};
    if(fault==='binding')info.frame.state.binding.generation='1';
    if(fault==='time')info.frame.state.time.ticks='1';
    if(fault==='width')info.page.scene.raster.width=2;
    if(fault==='bytes')info.page.scene.raster.byteLength='04';
    f.observed.badFrame={metadata:JSON.stringify({status:'rendered',info}),take_pixels:()=>new Uint8Array(4),free:()=>assert.fail('already consumed')};
    assert.throws(()=>owner.sample({ticks:'0',timescale:1},{}),PlaybackStateError);
    assert.equal(owner.closed,true);assert.equal(f.observed.free,1);
  }
});

function stepped(kind='author') {
  const f=fixture(kind), counts={begin:0,step:0,take:0,close:0,invalidate:0};
  const raster={beginRaster(){counts.begin++;if(f.beginError)throw f.beginError;
    if(f.beginStatus)return {status:f.beginStatus,pixels:new Uint8Array()};
    let steps=0;
    return {status:0,execution:{step(){counts.step++;f.reenter?.();steps++;
      return f.stepStatus?{status:f.stepStatus,pixels:new Uint8Array()}:{status:0,complete:steps>=2};},
      take(){counts.take++;if(f.takeError)throw f.takeError;return {status:0,pixels:Uint8Array.of(4,3,2,5)};},
      close(){counts.close++;if(f.closeError)throw f.closeError;}}};
  },invalidate(){counts.invalidate++;}};
  return Object.assign(f,{counts,raster});
}
const zero={ticks:'0',timescale:1};
for(const kind of ['author','source']) {
  test(`${kind}: stepped frame exclusively leases owner and consumes both handles once`,()=>{
    const f=stepped(kind),owner=f.open(),frame=owner.beginSample(zero,f.raster);
    for(const action of [()=>owner.sample(zero,{}),()=>owner.beginSample(zero,f.raster),()=>owner.timing(),
      ()=>owner.advance('9007199254740994'),()=>owner.dispose(),()=>owner.close()])
      assert.throws(action,e=>e.code==='BUSY');
    assert.throws(()=>frame.take(),/incomplete/);assert.throws(()=>frame.step(0),RangeError);
    f.reenter=()=>assert.throws(()=>frame.step(),e=>e.code==='BUSY');
    assert.equal(frame.step(),false);assert.equal(frame.step(7),false);
    assert.throws(()=>frame.take(),/incomplete/);
    f.observed.validationCallback=()=>assert.throws(()=>frame.step(),e=>e.code==='BUSY');
    assert.equal(frame.step(7),false);assert.equal(frame.step(7),true);
    assert.equal(frame.step(1),true);
    assert.deepEqual(frame.take().pixels,Uint8Array.of(4,3,2,5));
    assert(frame.closed);frame.close();assert.throws(()=>frame.take(),e=>e.code==='CLOSED');
    assert.equal(f.observed.pendingConsumed,1);assert.equal(f.observed.pendingFreed??0,0);
    assert.equal(f.counts.close,1);assert.equal(f.counts.invalidate,0);
    assert(!owner.closed);owner.dispose();assert.equal(f.observed.free,1);
  });
  test(`${kind}: cancellation before, during and after component completion releases without invalidation`,()=>{
    for(const steps of [0,1,2,3,4]) {
      const f=stepped(kind),owner=f.open(),frame=owner.beginSample(zero,f.raster);
      for(let n=0;n<steps;n++)frame.step();
      frame.close();frame.close();assert.equal(f.counts.close,1);
      assert.equal(f.observed.pendingFreed??0,steps<2?1:0);
      assert.equal(f.observed.validationFreed??0,steps>=2?1:0);
      assert.equal(f.observed.validationConsumed??0,0);
      assert.equal(f.observed.pendingConsumed??0,steps>=2?1:0);assert.equal(f.counts.invalidate,0);
      const next=owner.beginSample(zero,f.raster);while(!next.step()) {} next.take();
      owner.close();assert.equal(f.observed.free,1);
    }
  });
}
test('step/begin component failures preserve diagnostics and obey quarantine disposition',()=>{
  for(const stage of ['beginStatus','stepStatus'])for(const status of [1,2,3,4]) {
    const f=stepped(),owner=f.open();f[stage]=status;
    assert.throws(()=>{const frame=owner.beginSample(zero,f.raster);frame.step();},
      e=>e instanceof PlaybackComputationError&&e.invalidatesBackend===(status===2||status===4));
    assert.equal(owner.closed,status===2||status===4);assert.equal(f.counts.invalidate,status===2||status===4?1:0);
    assert.equal(f.observed.pendingConsumed,1);assert.equal(f.observed.pendingFreed??0,0);
    owner.close();assert.equal(f.observed.free,1);
  }
});
test('prepare errors release pending frame without touching raster; traps and cleanup failures remain terminal',()=>{
  const f=stepped(),owner=f.open();f.observed.prepareFailure=JSON.stringify(failure);
  assert.throws(()=>owner.beginSample(zero,f.raster),PlaybackComputationError);
  assert.equal(f.observed.pendingFreed,1);assert.equal(f.counts.begin,0);assert.equal(f.counts.invalidate,0);
  assert(!owner.closed);owner.close();
  for(const mode of ['begin','take','cleanup','metadata']) {
    const f=stepped(),owner=f.open();
    if(mode==='begin')f.beginError=Error('begin trap');
    if(mode==='take')f.takeError=Error('take trap');
    if(mode==='cleanup'){f.closeError=Error('component close');f.observed.pendingFreeError=Error('frame free');}
    if(mode==='metadata')f.observed.badFrame={metadata:'{',take_pixels(){assert.fail();},free(){f.observed.frameFree++;}};
    assert.throws(()=>{const task=owner.beginSample(zero,f.raster);if(mode==='cleanup')task.close();else{while(!task.step()) {} task.take();}},mode==='cleanup'?AggregateError:Error);
    assert(owner.closed);assert(f.counts.invalidate>0);assert.equal(f.observed.free,1);
    assert.equal((f.observed.pendingFreed??0)+(f.observed.pendingConsumed??0),1);
  }
});

test('validation traps and failed cleanup release each consumed handle and quarantine the backend',()=>{
  for(const mode of ['begin','step','complete','free']){
    const f=stepped(),owner=f.open();
    f.observed[{begin:'validationBeginError',step:'validationStepError',complete:'validationCompleteError',free:'validationFreeError'}[mode]]=Error('validation '+mode);
    assert.throws(()=>{
      const frame=owner.beginSample(zero,f.raster);
      frame.step();frame.step();
      if(mode==='free')frame.close();else {while(!frame.step()) {} frame.take();}
    });
    assert(owner.closed);assert.equal(f.observed.free,1);assert.equal(f.counts.invalidate,1);
    assert.equal(f.observed.pendingConsumed,1);assert.equal(f.observed.pendingFreed??0,0);
    assert.equal(f.counts.close,1);
    assert.equal((f.observed.validationFreed??0)+(f.observed.validationConsumed??0),mode==='begin'?0:1);
  }
});

test('observed validation faults quarantine immediately, even if a caller would abandon the frame',()=>{
  for(const stage of ['snapshot','step']){
    const f=stepped(),owner=f.open(),frame=owner.beginSample(zero,f.raster);
    if(stage==='snapshot')f.observed.validationFailed=true;
    else f.observed.validationCallback=()=>{f.observed.validationFailed=true;};
    assert.throws(()=>{while(!frame.step()) {}},e=>e instanceof PlaybackComputationError&&e.invalidatesBackend);
    assert(frame.closed);frame.close();assert(owner.closed);assert.equal(f.counts.invalidate,1);
    assert.equal(f.observed.validationConsumed,1);assert.equal(f.observed.validationFreed??0,0);
  }
});
