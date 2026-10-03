import assert from 'node:assert/strict';
import test from 'node:test';
import {PresentationEditorPage,EditorPageComputationError} from '../../.codex-work/editor-client/build/editor-client/src/index.js';
const view='1'.repeat(64);
const viewport={width:1,height:1,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:1},coordinateTolerance:'256',background:[0,0,0,0]};
const prepared={status:'prepared',view,info:{viewport,page:{sourceSha256:view,slide:'slide'}}};
const failure={status:'error',error:{stage:'request',error:{code:'INPUT_INVALID',message:'Rejected input'}}};
const request={input:{kind:'pptx'},fonts:{},page:{page:{viewport,expectedSourceSha256:view,slide:'slide'}}};
const inputs={material:new Uint8Array(),fonts:new Uint8Array(),decoder:{},shaping:{},raster:{}};
function fixture() {
 const state={ownerFreed:0,frameFreed:0,taken:0,onPrepare:()=>{},mode:'valid',inside:false};
 const owner={
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
  free(){assert.equal(state.inside,false,'never free an owner during a component callback');state.ownerFreed++;},
 };
 const client=new PresentationEditorPage({EditorPageSession:class {constructor(){return owner;}}});
 return {state,client};
}
test('typed rejection preserves prior owner and consumes the empty pixel envelope',()=>{
 const {state,client}=fixture();
 client.prepare(request,inputs);state.mode='typed-error';
 assert.throws(()=>client.prepare(request,inputs),EditorPageComputationError);
 assert.equal(client.view,view);assert.equal(client.closed,false);assert.equal(state.ownerFreed,0);
 assert.deepEqual(client.query([]),[]);assert.equal(state.taken,2);assert.equal(state.frameFreed,0);
 client.clear();assert.equal(client.view,null);client.close();client.close();assert.equal(state.ownerFreed,1);
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
