/** Diagnostic wrapper around the real packaged runtime. No alternate rendering. */
import {parentPort,workerData} from 'node:worker_threads';
import {performance} from 'node:perf_hooks';
let counts={},times={},maximum={},inputs={},rasterTaken=false;
const track=(name,body)=>{const t=performance.now();try{return body();}finally{const ms=performance.now()-t;counts[name]=(counts[name]??0)+1;times[name]=(times[name]??0)+ms;maximum[name]=Math.max(maximum[name]??0,ms);}};
globalThis.fetch=()=>{throw Error('Implicit fetch denied');};
const start=performance.now();
const {createPlaybackRuntime}=await import(workerData.entry);
const real=await createPlaybackRuntime(workerData.code);
const raster={invalidate:()=>real.raster.invalidate()};
for(const name of ['raster','rasterImages','decodeImage']) raster[name]=(...args)=>track(name,()=>real.raster[name](...args));
raster.beginRaster=(frame,images)=>track('componentBegin',()=>{
 inputs.frame=(inputs.frame??0)+frame.byteLength;inputs.images=(inputs.images??0)+(images?.byteLength??0);
 const result=real.raster.beginRaster(frame,images);if(result.status!==0)return result;
 const task=result.execution;return {status:0,execution:{step:n=>track('componentStep',()=>task.step(n)),take:()=>track('componentTake',()=>{rasterTaken=true;return task.take();}),close:()=>track('componentClose',()=>task.close())}};
});
const shaping={invalidate:()=>real.shaping.invalidate()};
for(const name of ['shapeBatch','outlineBatch','measureBatch'])shaping[name]=(...args)=>track(name,()=>real.shaping[name](...args));
const {createDispatcher}=await import(workerData.dispatch);
const dispatch=createDispatcher({...real,raster,shaping});
parentPort.postMessage({ready:true,initializationMs:performance.now()-start});
let busy=false;
parentPort.on('message',async({id,q})=>{
 if(busy)throw Error('Diagnostic worker accepts one command');busy=true;
 counts={};times={};maximum={};inputs={};const t=performance.now();let yields=0;const stages={};
 try{
  let value;
  if(q.operation==='localSample'){
   rasterTaken=false;
   let start=performance.now();dispatch({operation:'beginSample',at:q.at,history:q.history});stages.beginMs=performance.now()-start;
   let slice=performance.now();stages.stepSumMs=0;stages.maxStepMs=0;stages.validationSumMs=0;stages.validationMaxMs=0;stages.steps=0;
   for(;;){
    const validating=rasterTaken;start=performance.now();const complete=dispatch({operation:'stepSample',workUnits:q.workUnits}).complete;
    const ms=performance.now()-start;stages.stepSumMs+=ms;stages.maxStepMs=Math.max(stages.maxStepMs,ms);++stages.steps;
    if(validating){stages.validationSumMs+=ms;stages.validationMaxMs=Math.max(stages.validationMaxMs,ms);}
    if(complete)break;
    if(stages.steps>1000000)throw Error('Diagnostic step limit');
    if(q.yieldMs&&performance.now()-slice>=q.yieldMs){++yields;await new Promise(setImmediate);slice=performance.now();}
   }
   start=performance.now();value=dispatch({operation:'takeSample'});stages.takeMs=performance.now()-start;
  }else value=dispatch(q);
  const workMs=performance.now()-t;
  parentPort.postMessage({id,value,measurement:{counts,times,maximum,inputs,workMs,yields,stages}},value?.pixels?[value.pixels.buffer]:[]);
 }catch(e){parentPort.postMessage({id,error:{name:e.name,message:e.message,code:e.code,stack:e.stack}});}
 finally{busy=false;}
});
