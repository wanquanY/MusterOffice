import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
const [nativePath,wasmPath,reportPath]=process.argv.slice(2)
assert(reportPath,'usage: video-export-parity.mjs native wasm.js report.json')
const wasm=createRequire(import.meta.url)(resolve(wasmPath))
const sha=v=>createHash('sha256').update(v).digest('hex')
const cases=[]
let comparedFrames=0,comparedPcmSamples=0
for(const [rate,audio] of [[48000,true],[44100,true],[48000,false]]) {
  let first
  for(const chunk of [997,4093]) {
    const raw=JSON.stringify({rate,audio,chunk})
    const native=spawnSync(resolve(nativePath),['--export'],{input:raw,encoding:'utf8',maxBuffer:8<<20})
    assert.equal(native.status,0,native.stderr)
    const result=JSON.parse(native.stdout)
    assert.equal(result.status,'complete',result.error)
    assert.deepEqual(JSON.parse(wasm.video_export_conformance(raw)),result)
    assert.equal(result.videoClock,240000)
    assert.equal(result.video.length,16)
    assert.deepEqual(result.video.map(f=>f.pts),Array.from({length:16},(_,i)=>i*8008))
    assert.equal(result.video.at(-1).duration,665)
    assert.equal(result.video[0].planes[0][0],63)
    assert.equal(result.video[12].planes[0][0],47)
    assert.equal(result.video[14].planes[0][0],173)
    assert.equal(result.audioSamples,audio?Math.ceil(24157/48000*rate):0)
    assert.equal(result.pcmBits.length,result.audioSamples*2)
    if(audio) assert(new Set(result.pcmBits).size>100,'must mix real varied PCM')
    for(const spatial of result.spatial) {
      assert.equal(spatial.planes.reduce((n,p)=>n+p.length,0),spatial.width*spatial.height*1.5)
      assert.equal(spatial.scratch,spatial.width*80)
    }
    if(first) assert.deepEqual(result,first,'output must not depend on audio partition')
    else first=result
    cases.push({rate,audio,chunk,resultSha256:sha(JSON.stringify(result))})
    comparedFrames+=result.video.length
    comparedPcmSamples+=result.pcmBits.length
  }
}
const invalid=JSON.stringify({rate:48000,audio:true,chunk:0})
const native=spawnSync(resolve(nativePath),['--export'],{input:invalid,encoding:'utf8'})
assert.equal(native.status,0,native.stderr)
assert.deepEqual(JSON.parse(wasm.video_export_conformance(invalid)),JSON.parse(native.stdout))
assert.equal(JSON.parse(native.stdout).status,'invalidFixture')
const report={format:'musteroffice.video-export-parity/1',status:'passed',cases,comparedFrames,comparedPcmSamples,spatialVectors:3,
  nativeSha256:sha(readFileSync(resolve(nativePath))),wasmSha256:sha(readFileSync(resolve(wasmPath.replace(/\.js$/,'_bg.wasm')))),
  node:process.version,platform:process.platform,arch:process.arch,
  scope:'Exact native/WASM encoder input planes, PCM bits, clocks, partition independence and nonuniform SDR/HDR I420 conversion. Test capture only; actual native files checked separately. No browser codec or product acceptance.'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report))
