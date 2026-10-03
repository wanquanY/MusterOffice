import assert from 'node:assert/strict';
import test from 'node:test';
import {PresentationEditorPage,EditorPageComputationError} from '../../.codex-work/editor-client/build/editor-client/src/index.js';
const view='1'.repeat(64);
const viewport={width:1,height:1,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:1},coordinateTolerance:'256',background:[0,0,0,0]};
const objects=[2,3].map(nativeId=>({object:{part:'ppt/slides/slide1.xml',nativeId}}));
const prepared={status:'prepared',view,info:{viewport,page:{sourceSha256:view,slide:'slide'},objects,
 textFrames:objects.map((o,frame)=>({frame,object:o.object}))}};
const queries=[{device:{point:{x:'0',y:'0'},radius:'0'},maxHits:2}];
const picked={status:'picked',view,results:[{hits:[{object:0,kind:'exact',textFrame:0}],truncated:false}]};
const failure={status:'error',error:{stage:'request',error:{code:'INPUT_INVALID',message:'Rejected input'}}};
const request={input:{kind:'pptx'},fonts:{},page:{page:{viewport,expectedSourceSha256:view,slide:'slide'}}};
const inputs={material:new Uint8Array(),fonts:new Uint8Array(),decoder:{},shaping:{},raster:{}};
const inspected={status:'inspected',info:{sourceSha256:view,model:null,pageSize:null,
 slides:[{slide:'slide',nativeId:256,slideId:null,name:null,hidden:false}]}};
function fixture() {
 const state={ownerFreed:0,frameFreed:0,taken:0,onPrepare:()=>{},onPick:()=>{},onInspect:()=>{},inspectReply:structuredClone(inspected),pickReply:structuredClone(picked),mode:'valid',inside:false};
 const owner={
  inspect(json,material){
   state.inside=true;
   try {assert.equal(JSON.parse(json).operation,'inspect');state.onInspect(material);return JSON.stringify(state.inspectReply);}
   finally {state.inside=false;}
  },
  prepare(){
   state.inside=true;
   try {
    state.onPrepare();
    return {
     get metadata(){if(state.mode==='getter-trap')throw new Error('getter trap');return JSON.stringify(state.mode==='typed-error'?failure:prepared);},
     take_pixels(){state.taken++;if(state.mode==='take-trap')throw new Error('take trap');return new Uint8Array(state.mode==='typed-error'?0:state.mode==='short'?3:4);},
     free(){state.frameFreed++;},
    };
   } finally {state.inside=false;}
  },
  command(json){const q=JSON.parse(json);return JSON.stringify(q.operation==='clear'?{status:'cleared',view}:{status:'queried',view,results:[]});},
  pick(json,raster){
   state.inside=true;
   try {assert.deepEqual(JSON.parse(json),{operation:'pick',view,queries});state.onPick(raster);return JSON.stringify(state.pickReply);}
   finally {state.inside=false;}
  },
  free(){assert.equal(state.inside,false,'never free an owner during a component callback');state.ownerFreed++;},
 };
 const client=new PresentationEditorPage({EditorPageSession:class {constructor(){return owner;}}});
 return {state,client};
}
test('discovery requires no page or components, and preserves the current rendered view on success or rejection',()=>{
 const {state,client}=fixture();const material=new Uint8Array([1]);
 state.onInspect=bytes=>assert.equal(bytes,material);
 assert.deepEqual(client.inspect({kind:'pptx'},material),inspected.info);assert.equal(client.view,null);
 client.prepare(request,inputs);assert.deepEqual(client.inspect({kind:'pptx'},material),inspected.info);assert.equal(client.view,view);
 state.inspectReply=failure;assert.throws(()=>client.inspect({kind:'pptx'},material),EditorPageComputationError);
 assert.equal(client.closed,false);assert.equal(client.view,view);assert.deepEqual(client.query([]),[]);
 client.close();assert.equal(state.ownerFreed,1);
});
test('discovery validates native uniqueness and ordered model correspondence',()=>{
 const input={kind:'author',document:{id:'doc',slideOrder:['b','a'],pageSize:{width:'1',height:'2'}},defaults:{}};
 const valid={status:'inspected',info:{sourceSha256:view,model:{id:'doc',semanticDigest:view},pageSize:input.document.pageSize,
  slides:['b','a'].map((slideId,i)=>({slide:`part${i}`,nativeId:256+i,slideId,name:slideId,hidden:i===1}))}};
 const changes=[
  r=>{r.status='prepared';},r=>{r.info.sourceSha256='bad';},r=>{r.info.model=null;},r=>{r.info.model.id='other';},
  r=>{r.info.model.semanticDigest='bad';},r=>{r.info.pageSize=null;},r=>{r.info.pageSize.width='3';},
  r=>{r.info.slides.pop();},r=>{r.info.slides.reverse();},r=>{r.info.slides[1].slide=r.info.slides[0].slide;},
  r=>{r.info.slides[1].nativeId=r.info.slides[0].nativeId;},r=>{r.info.slides[0].nativeId=-1;},
  r=>{r.info.slides[0].nativeId=0x100000000;},r=>{r.info.slides[0].slideId=null;},r=>{r.info.slides[0].hidden='false';},
 ];
 const good=fixture();good.state.inspectReply=valid;assert.deepEqual(good.client.inspect(input),valid.info);good.client.close();
 for(const change of changes){
  const {state,client}=fixture();state.inspectReply=structuredClone(valid);change(state.inspectReply);
  assert.throws(()=>client.inspect(input));assert.equal(client.closed,true);assert.equal(state.ownerFreed,1);
 }
});
test('discovery observes the same busy and deferred-close ownership discipline',()=>{
 const {state,client}=fixture();client.prepare(request,inputs);
 state.onInspect=()=>{assert.throws(()=>client.inspect({kind:'pptx'}),/closed or busy/);};
 client.inspect({kind:'pptx'});assert.equal(client.view,view);
 state.onInspect=()=>{client.close();assert.equal(state.ownerFreed,0);};
 assert.throws(()=>client.inspect({kind:'pptx'}),/closed during calculation/);assert.equal(state.ownerFreed,1);
 assert.equal(client.view,null);client.close();assert.equal(state.ownerFreed,1);
});
test('typed rejection preserves prior owner and consumes the empty pixel envelope',()=>{
 const {state,client}=fixture();
 client.prepare(request,inputs);state.mode='typed-error';
 assert.throws(()=>client.prepare(request,inputs),EditorPageComputationError);
 assert.equal(client.view,view);assert.equal(client.closed,false);assert.equal(state.ownerFreed,0);
 assert.deepEqual(client.query([]),[]);assert.equal(state.taken,2);assert.equal(state.frameFreed,0);
 client.clear();assert.equal(client.view,null);client.close();client.close();assert.equal(state.ownerFreed,1);
});
test('picking uses the current owner, preserves typed rejection and refuses a cleared view',()=>{
 const {state,client}=fixture(),raster={};
 assert.throws(()=>client.pick(queries,raster),/not been prepared/);
 client.prepare(request,inputs);state.onPick=port=>assert.equal(port,raster);
 assert.deepEqual(client.pick(queries,raster),picked.results);
 state.pickReply=failure;assert.throws(()=>client.pick(queries,raster),EditorPageComputationError);
 assert.equal(client.closed,false);assert.equal(client.view,view);assert.equal(state.ownerFreed,0);
 state.pickReply=picked;assert.deepEqual(client.pick(queries,raster),picked.results);
 client.clear();assert.throws(()=>client.pick(queries,raster),/not been prepared/);
 client.close();assert.equal(state.ownerFreed,1);
});
test('picking cannot free an owner during its component call or publish a late result',()=>{
 const {state,client}=fixture();client.prepare(request,inputs);
 state.onPick=()=>{client.close();assert.equal(state.ownerFreed,0);};
 assert.throws(()=>client.pick(queries,{}),/closed during calculation/);
 assert.equal(state.ownerFreed,1);assert.equal(client.view,null);client.close();assert.equal(state.ownerFreed,1);
});
test('reentrant picking is refused while its active owner remains valid',()=>{
 const {state,client}=fixture();client.prepare(request,inputs);
 state.onPick=()=>{assert.throws(()=>client.pick(queries,{}),/closed or busy/);assert.equal(state.ownerFreed,0);};
 assert.deepEqual(client.pick(queries,{}),picked.results);assert.equal(client.closed,false);client.close();
});
test('malformed picking replies quarantine the owner including foreign text-frame bindings',()=>{
 const changes=[
  r=>{r.status='queried';}, r=>{r.view='2'.repeat(64);}, r=>{r.results=[];},
  r=>{r.results[0].truncated='false';}, r=>{r.results[0].truncated=true;},
  r=>{r.results[0].hits[0].object=2;}, r=>{r.results[0].hits[0].object=-1;},
  r=>{r.results[0].hits[0].object=0.5;}, r=>{r.results[0].hits[0].kind='paint';},
  r=>{r.results[0].hits[0].textFrame=1;}, r=>{r.results[0].hits[0].textFrame=2;},
  r=>{r.results[0].hits.push(r.results[0].hits[0]);},
  r=>{r.results[0].hits.push(r.results[0].hits[0],r.results[0].hits[0]);},
 ];
 for(const change of changes){
  const {state,client}=fixture();client.prepare(request,inputs);change(state.pickReply);
  assert.throws(()=>client.pick(queries,{}));assert.equal(client.closed,true);
  assert.equal(client.view,null);assert.equal(state.ownerFreed,1);client.close();assert.equal(state.ownerFreed,1);
 }
});
test('picking bridge traps release the owner once after returning from the callback',()=>{
 const {state,client}=fixture();client.prepare(request,inputs);state.onPick=()=>{throw new Error('bridge trap');};
 assert.throws(()=>client.pick(queries,{}),/bridge trap/);assert.equal(client.closed,true);assert.equal(state.ownerFreed,1);
 client.close();assert.equal(state.ownerFreed,1);
});
test('getter and consuming transfer traps release the correct allocations once',()=>{
 for(const mode of ['getter-trap','take-trap','short']) {
  const {state,client}=fixture();state.mode=mode;
  assert.throws(()=>client.prepare(request,inputs));assert.equal(client.closed,true);assert.equal(client.view,null);
  assert.equal(state.ownerFreed,1);assert.equal(state.frameFreed,mode==='getter-trap'?1:0);
  assert.equal(state.taken,mode==='getter-trap'?0:1);client.close();assert.equal(state.ownerFreed,1);
 }
});
test('close from a component callback defers owner release and discards the late result',()=>{
 const {state,client}=fixture();
 state.onPrepare=()=>{client.close();assert.equal(state.ownerFreed,0);};
 assert.throws(()=>client.prepare(request,inputs),/closed during calculation/);
 assert.equal(state.ownerFreed,1);assert.equal(state.taken,1);assert.equal(client.view,null);
 assert.throws(()=>client.prepare(request,inputs),/closed or busy/);
});
test('reentrant operations are refused without freeing the active Rust borrow',()=>{
 const {state,client}=fixture();
 state.onPrepare=()=>{assert.throws(()=>client.prepare(request,inputs),/closed or busy/);assert.equal(state.ownerFreed,0);};
 client.prepare(request,inputs);assert.equal(client.closed,false);assert.equal(client.view,view);
 client.close();assert.equal(state.ownerFreed,1);
});
