import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
const [nativePath, wasmPath, reportPath] = process.argv.slice(2)
assert(nativePath && wasmPath && reportPath, 'usage: video-audio-parity.mjs native wasm.js report.json')
const wasm=createRequire(import.meta.url)(resolve(wasmPath))
const t=(n,d=8000)=>({ticks:String(n),timescale:d})
const ratio=(n,d=1)=>({numerator:String(n),denominator:String(d)})
const range=(a,b)=>({start:t(a),end:t(b)})
const map=(a,b,n)=>({anchors:[{local:t(0),source:t(a)},{local:t(n),source:t(b)}]})
const policy={rate:'resample',reverse:'reverse',hold:'silence'}
const media=(a,b,n)=>({kind:'media',resource:'sound',stream:'a0',timeMap:map(a,b,n)})
const clip=(id,a,b,source,effects=[])=>({id,name:id,timeline:range(a,b),source,effects,audio:policy})
const track=(id,clips)=>({id,name:id,kind:'audio',clips,muted:false,solo:false,locked:false})
const document={format:'musteroffice.video/1',id:'film',title:'Audio parity',canvas:{width:100,height:100},frameRate:{numerator:25,denominator:1},audioSampleRate:8000,rootSequence:'main',linkGroups:{},
  sequences:{main:{id:'main',tracks:['music','speech','outer']},child:{id:'child',tracks:['inner']}},
  tracks:{music:track('music',['speed']),speech:track('speech',['reverse','preserve']),outer:track('outer',['group']),inner:track('inner',['child-voice'])},
  clips:{
    speed:clip('speed',0,2048,media(0,3072,2048),[{kind:'gain',milliDecibels:-1234},{kind:'fade',fadeIn:t(777),fadeOut:t(300),envelope:range(-233,2048)}]),
    reverse:clip('reverse',0,1024,media(2048,1024,1024)),
    preserve:{...clip('preserve',3072,4096,media(0,2048,1024)),audio:{...policy,rate:'preservePitch'}},
    group:clip('group',100,1124,{kind:'sequence',sequence:'child',timeMap:map(1024,0,1024)},[{kind:'gain',milliDecibels:-6000}]),
    'child-voice':clip('child-voice',0,1024,media(1024,0,1024)),
  },resources:{sound:{id:'sound',digest:'13'.repeat(32),byteLength:'100',descriptor:{kind:'media',streams:{a0:{kind:'audio',available:range(0,4096),sampleRate:8000,channels:1}}}}}}
// Integer arithmetic generates identical input PCM without relying on JS/native sin agreement.
const pcm=Array.from({length:4096},(_,i)=>((i*1637)%64000-32000)/32768)
const fixture={document,pcm:[{resource:'sound',stream:'a0',sampleZero:ratio(0),firstSample:'0',samples:pcm}],queries:[{first:'0',frames:2048},{first:'123',frames:137},{first:'260',frames:1000},{first:'2000',frames:96},{first:'2500',frames:50},{first:'3072',frames:100},{first:'4096',frames:1},{first:'-1',frames:10}]}
const raw=JSON.stringify(fixture)
const native=spawnSync(resolve(nativePath),['--audio'],{input:raw,encoding:'utf8',maxBuffer:8<<20})
assert.equal(native.status,0,native.stderr)
const result=JSON.parse(native.stdout)
assert.equal(result.status,'complete',result.error)
assert.deepEqual(JSON.parse(wasm.video_audio_conformance(raw)),result)
assert.deepEqual(result.results.map(r=>r.status),['mixed','mixed','mixed','mixed','mixed','rejected','rejected','rejected'])
assert.deepEqual(result.results[1].bits,result.results[0].bits.slice(123,260))
assert.deepEqual(result.results[2].bits,result.results[0].bits.slice(260,1260))
assert(result.results[4].bits.every(n=>n===0))
assert(result.results[5].error.includes('preserve-pitch'))
const sha=v=>createHash('sha256').update(v).digest('hex')
const report={format:'musteroffice.video-audio-parity/1',status:'passed',queries:8,mixed:5,rejected:3,profile:result.profile,fixtureSha256:sha(raw),resultSha256:sha(JSON.stringify(result)),nativeSha256:sha(readFileSync(resolve(nativePath))),wasmSha256:sha(readFileSync(resolve(wasmPath.replace(/\.js$/,'_bg.wasm')))),node:process.version,platform:process.platform,arch:process.arch,scope:'Actual native/WASM block PCM bits, spans, automation and nested DSP. Not browser audio device or full time-stretch quality.'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report))
