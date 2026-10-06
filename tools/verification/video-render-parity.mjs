import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { writeFileSync, readFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
const [nativePath, wasmPath, reportPath] = process.argv.slice(2)
assert(nativePath && wasmPath && reportPath, 'usage: video-render-parity.mjs native wasm.js report.json')
const wasm = createRequire(import.meta.url)(resolve(wasmPath))
const time = {time:{numerator:'0',denominator:'1'},side:'after'}
const frame = (format,width,height,colors,planes,extra={}) => ({info:{generation:1,kind:'video',timeBase:{numerator:1,denominator:30},ptsTicks:0,durationTicks:1,width,height,sampleRate:null,channels:null,samples:null,format,channelLayout:'',colors,sampleAspectRatio:{numerator:1,denominator:1},alphaMode:2,keyFrame:true,interlaced:false,topFieldFirst:false},planes,hasAlpha:format==='rgba',hlgDisplay:false,toneMap:false,...extra})
const plane = (rowBytes,rows,bytes) => ({rowBytes,rows,bytes})
const yuv = {primaries:1,transfer:1,matrix:1,range:1,chromaLocation:1}
const rgb = {primaries:1,transfer:13,matrix:0,range:2,chromaLocation:0}
const solid = (clip,rgba) => ({track:'v1',clip,local:time,effects:[],source:{kind:'solid',rgba}})
const group = {track:'v1',clip:'group',local:time,effects:[{kind:'opacity',factor:{numerator:'1',denominator:'2'}}],source:{kind:'group',sequence:'child',mapping:{sample:time,rate:{numerator:'1',denominator:'1'}},layers:[solid('red',[255,0,0,255]),solid('blue',[0,0,255,255])]}}
const plan = (layers) => ({documentDigest:'11'.repeat(32),frameIndex:'0',sample:time,canvas:{width:4,height:4},layers})
const transformed = solid('rotate',[255,30,60,128])
transformed.effects.push({kind:'transform',xMilliPixels:500,yMilliPixels:0,scaleXPpm:750000,scaleYPpm:750000,rotationMilliDegrees:23000,opacityPpm:700000})
const fixture = {frames:[
  frame('rgba',2,1,rgb,[plane(8,1,[255,0,0,128,0,255,0,0])]),
  frame('yuv420p',3,3,yuv,[plane(3,3,[16,64,235,128,180,100,20,220,160]),plane(2,2,[80,190,40,150]),plane(2,2,[128,100,140,120])]),
  frame('p010le',2,2,yuv,[plane(4,2,[0,16,0,235,0,16,0,235]),plane(4,1,[0,128,0,128])]),
  frame('rgba',2,1,{...rgb,primaries:9,transfer:16},[plane(8,1,[128,150,180,255,200,180,240,128])],{toneMap:true}),
  frame('rgba',1,1,{...rgb,primaries:9,transfer:18},[plane(4,1,[128,191,255,255])],{hlgDisplay:true,toneMap:true}),
  frame('rgba',1,1,{...rgb,primaries:9,transfer:16},[plane(4,1,[128,128,128,255])]),
  frame('rgba',1,1,{...rgb,primaries:2},[plane(4,1,[0,0,0,255])]),
],plans:[plan([solid('white',[255,255,255,255]),group]),plan([solid('dark',[10,20,30,255]),transformed])]}
const raw=JSON.stringify(fixture)
const native=spawnSync(resolve(nativePath),['--render'],{input:raw,encoding:'utf8',maxBuffer:4*1024*1024})
assert.equal(native.status,0,native.stderr)
const result=JSON.parse(native.stdout)
assert.equal(result.status,'complete',result.error)
assert.deepEqual(JSON.parse(wasm.video_render_conformance(raw)),result)
assert.deepEqual(result.results.map(r=>r.status),['rendered','rendered','rendered','rendered','rendered','rejected','rejected','rendered','rendered'])
assert.deepEqual(result.results[0].value.rgba,[255,0,0,128,0,0,0,0])
assert.deepEqual(result.results[2].value.rgba,[0,0,0,255,255,255,255,255,0,0,0,255,255,255,255,255])
for(let i=0;i<16;i++) assert.deepEqual(result.results[7].value.rgba.slice(i*4,i*4+4),[188,188,255,255])
const sha = value => createHash('sha256').update(value).digest('hex')
const report={format:'musteroffice.video-render-parity/1',status:'passed',queries:9,rendered:7,rejected:2,fixtureSha256:sha(raw),resultSha256:sha(JSON.stringify(result)),nativeSha256:sha(readFileSync(resolve(nativePath))),wasmSha256:sha(readFileSync(resolve(wasmPath.replace(/\.js$/, '_bg.wasm')))),scope:'Actual native/WASM CPU pixel vectors, including exact float bits. Not browser decode, GPU or product acceptance.'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report))
