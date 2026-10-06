/** Actual Rust Native/WASM scheduling and independently instantiated C++ DSP. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const [binary, rustWasm, componentJs, componentWasm, adapterJs, reportPath] = process.argv.slice(2);
assert(reportPath, 'usage: native rust-wasm.js component.mjs component.wasm adapter.js report.json');
const digest = value => createHash('sha256').update(value).digest('hex');
const componentRoot=path.resolve('components/signalsmith-stretch'), buildDirectory=path.dirname(path.resolve(componentJs));
const build=JSON.parse(fs.readFileSync(path.join(buildDirectory,'build.json'),'utf8'));
assert.equal(build.format,'musteroffice.stretch-build/1');assert.equal(build.target,'wasm');
assert.equal(build.faultTests,false);assert.equal(build.sanitizers,false);
assert.equal(build.builder.path,'tools/components/build-stretch.py');
assert.equal(digest(fs.readFileSync(build.builder.path)),build.builder.sha256);
assert.deepEqual(build.lock,JSON.parse(fs.readFileSync(path.join(componentRoot,'lock.json'),'utf8')));
for(const [base,rows] of [[componentRoot,build.sources],[buildDirectory,build.artifacts],[buildDirectory,build.preparedSources]])
  for(const row of rows) {
    assert(!path.isAbsolute(row.path) && row.path.split(/[\\/]/).every(s=>s && s!=='.' && s!=='..'));
    const data=fs.readFileSync(path.join(base,row.path));assert.equal(data.length,row.byteLength);assert.equal(digest(data),row.sha256);
  }
assert.deepEqual(build.sources.map(s=>s.path).sort(),['mo_stretch.cpp','mo_stretch.h','mo_stretch_memory.h']);
assert.deepEqual(build.artifacts.map(s=>path.resolve(buildDirectory,s.path)).sort(),[path.resolve(componentJs),path.resolve(componentWasm)].sort());
const rust = await import(pathToFileURL(path.resolve(rustWasm)));
const {default:factory} = await import(pathToFileURL(path.resolve(componentJs)));
const {StretchComponent} = await import(pathToFileURL(path.resolve(adapterJs)));
const compiled = new WebAssembly.Module(fs.readFileSync(componentWasm));
const runNative = fixture => JSON.parse(execFileSync(binary, ['--stretch'], {input:JSON.stringify(fixture), maxBuffer:64<<20}));
function sine(frames, rate, channels = 1) {
  return Array.from({length:frames*channels}, (_, i) => Math.fround(.3*Math.sin(2*Math.PI*(i%channels ? 990 : 440)*Math.floor(i/channels)/rate)));
}
const tone = sine(96000,48000);
const cases = [
  ...[48000,64000,128000,192000,288000].map(n => ({name:`tone-96000-to-${n}`, sampleRate:48000, channels:1, samples:tone, outputFrames:n})),
  {name:'stereo',sampleRate:48000,channels:2,samples:sine(48000,48000,2),outputFrames:72000},
  {name:'44100',sampleRate:44100,channels:1,samples:sine(44100,44100),outputFrames:60001},
  {name:'silence',sampleRate:48000,channels:1,samples:Array(4000).fill(0),outputFrames:9000},
  {name:'tail',sampleRate:48000,channels:1,samples:Array.from({length:48000},(_,i)=>i>=45000?.25:0),outputFrames:96000},
  ...[1,16,128].map(n=>({name:`short-${n}`,sampleRate:48000,channels:1,samples:Array(n).fill(.25),outputFrames:n*2})),
];
const results=[];
for (const {name,...fixture} of cases) {
  const native=runNative(fixture);
  const component=await StretchComponent.create(factory,compiled,fixture.sampleRate,fixture.channels);
  let wasm,memory;
  try { wasm=JSON.parse(rust.video_stretch_conformance(JSON.stringify(fixture),component)); memory=component.memory(); }
  finally { component.dispose(); }
  assert.equal(native.status,'complete',`${name}: ${JSON.stringify(native)}`);
  assert.equal(wasm.status,'complete',`${name}: ${JSON.stringify(wasm)}`);
  assert.equal(native.samples.length,fixture.outputFrames*fixture.channels);
  assert.equal(wasm.samples.length,native.samples.length);
  assert.deepEqual(wasm.chunks,native.chunks);
  assert.equal(wasm.inputLatency,native.inputLatency);
  assert.equal(wasm.outputLatency,native.outputLatency);
  let maximum=0,square=0,signal=0;
  for(let i=0;i<native.samples.length;i++) {
    assert(Number.isFinite(wasm.samples[i]));
    const d=wasm.samples[i]-native.samples[i]; maximum=Math.max(maximum,Math.abs(d));square+=d*d;signal+=native.samples[i]**2;
  }
  const rms=Math.sqrt(square/native.samples.length);
  const snr=square>0?10*Math.log10(signal/square):null;
  const nativeBits=Buffer.from(new Float32Array(native.samples).buffer), wasmBits=Buffer.from(new Float32Array(wasm.samples).buffer);
  const bitIdentical=nativeBits.equals(wasmBits);
  assert(maximum<=2e-4 && rms<=2e-5,`${name}: max=${maximum}, RMS=${rms}, SNR=${snr}`);
  if(name==='silence') assert(native.samples.every(v=>v===0) && wasm.samples.every(v=>v===0));
  if(name.startsWith('short')) assert(wasm.samples.some(v=>Math.abs(v)>1e-8));
  if(name==='tail') {
    assert(wasm.samples.slice(92000).reduce((a,v)=>a+Math.abs(v),0)>100);
    assert(wasm.samples.slice(0,80000).every(v=>Math.abs(v)<1e-5));
  }
  if(name.startsWith('tone') || name==='stereo' || name==='44100') {
    // Pitch and energy measured independently from the scheduling implementation.
    for(let ch=0;ch<fixture.channels;ch++) {
      const edge=Math.round(fixture.sampleRate*.2), expected=ch?990:440;
      let crossing=0, energy=0;
      for(let i=edge;i<fixture.outputFrames-edge;i++) {
        const v=wasm.samples[i*fixture.channels+ch]; energy+=v*v;
        if(v>0 && wasm.samples[(i-1)*fixture.channels+ch]<=0) crossing++;
      }
      const length=fixture.outputFrames-2*edge, hz=crossing*fixture.sampleRate/length;
      assert(Math.abs(hz-expected)<3,`${name} channel ${ch}: ${hz}Hz`);
      assert(Math.sqrt(energy/length)>.15 && Math.sqrt(energy/length)<.26,`${name}: RMS energy`);
    }
  }
  // Inserting source-read retries cannot reset/advance the corresponding DSP step.
  const retryNative=runNative({...fixture,retryReads:true});
  assert.deepEqual(retryNative.samples,native.samples); assert(retryNative.retries>0);
  const retryComponent=await StretchComponent.create(factory,compiled,fixture.sampleRate,fixture.channels);
  let retryWasm;
  try {retryWasm=JSON.parse(rust.video_stretch_conformance(JSON.stringify({...fixture,retryReads:true}),retryComponent));}
  finally {retryComponent.dispose();}
  assert.deepEqual(retryWasm.samples,wasm.samples); assert(retryWasm.retries>0);
  results.push({name,frames:fixture.outputFrames,channels:fixture.channels,maximum,rms,snr,bitIdentical,memory,
    nativeSha256:digest(JSON.stringify(native.samples)),wasmSha256:digest(JSON.stringify(wasm.samples)),
    fixtureSha256:digest(JSON.stringify(fixture)),retries:retryNative.retries});
}
// Adapter validation and lifetime work against the real module, not a mock.
const component=await StretchComponent.create(factory,compiled,48000,1);
assert.throws(()=>component.begin(new Float32Array(component.inputLatency).fill(NaN),1));
component.begin(new Float32Array(component.inputLatency),1);
assert.throws(()=>component.begin(new Float32Array(component.inputLatency),1));
assert.throws(()=>component.process(new Float32Array([Infinity]),new Float32Array(128)));
const silence=new Float32Array(128).fill(1);
component.process(new Float32Array(128),silence);assert(silence.every(v=>v===0));
component.dispose();component.dispose();
assert.throws(()=>component.process(new Float32Array(128),silence));
const report={format:'musteroffice.stretch-parity/1',runtime:process.version,
  artifacts:[binary,rustWasm,componentJs,componentWasm,adapterJs].map(p=>({path:p,sha256:digest(fs.readFileSync(p))})),
  tolerance:{maximum:2e-4,rms:2e-5},cases:results};
fs.mkdirSync(path.dirname(reportPath),{recursive:true});fs.writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({cases:results.length,bitIdentical:results.every(v=>v.bitIdentical),maximum:Math.max(...results.map(v=>v.maximum)),maxRms:Math.max(...results.map(v=>v.rms)),report:reportPath}));
