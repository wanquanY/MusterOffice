import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { resolve } from 'node:path'

const [nativePath, wasmPath, reportPath] = process.argv.slice(2)
assert(nativePath && wasmPath && reportPath, 'usage: node video-projection-parity.mjs native wasm.js report.json')
const wasm = createRequire(import.meta.url)(resolve(wasmPath))
const t = (n,d=1) => ({ticks:String(n),timescale:d})
const ratio = (n,d=1) => ({numerator:String(n),denominator:String(d)})
const sample = (n,d=1,side='after') => ({time:ratio(n,d),side})
const range = (a,b,d=1) => ({start:t(a,d),end:t(b,d)})
const map = (a,b,d) => ({anchors:[{local:t(0),source:t(a)},{local:t(d),source:t(b)}]})
const policy = {rate:'preservePitch',reverse:'reverse',hold:'silence'}
const media = (resource,stream,a,b,d) => ({kind:'media',resource,stream,timeMap:map(a,b,d)})
const nested = (sequence,a,b,d) => ({kind:'sequence',sequence,timeMap:map(a,b,d)})
const clip = (id,a,b,source,effects=[]) => ({id,name:id,timeline:range(a,b),source,effects,audio:policy})
const track = (id,kind,clips) => ({id,name:id,kind,clips,muted:false,solo:false,locked:false})
const dimensions = {width:1920,height:1080}
const movie = {id:'movie',digest:'13'.repeat(32),byteLength:'100',descriptor:{kind:'media',streams:{
  v0:{kind:'video',available:range(0,4),dimensions,frameRate:null},
  a0:{kind:'audio',available:range(0,4),sampleRate:44100,channels:2},
}}}
// Synthetic metadata proves domain semantics only; it is deliberately not a media decode fixture.
const document = {
  format:'musteroffice.video/1',id:'film',title:'投影 · 逆向组合',canvas:dimensions,
  frameRate:{numerator:30000,denominator:1001},audioSampleRate:48000,rootSequence:'main',linkGroups:{},
  sequences:{main:{id:'main',tracks:['video','graphics','sound']},child:{id:'child',tracks:['inner','voice']}},
  tracks:{video:track('video','video',['group']),graphics:track('graphics','graphics',['text','solid','image']),
    sound:track('sound','audio',['audio-group','hold','mute']),inner:track('inner','video',['first','last']),voice:track('voice','audio',['narration'])},
  clips:{
    group:clip('group',0,4,nested('child',4,0,4),[
      {kind:'transform',xMilliPixels:0,yMilliPixels:0,scaleXPpm:1000000,scaleYPpm:1000000,rotationMilliDegrees:0,opacityPpm:500000},
      {kind:'fade',fadeIn:t(1),fadeOut:t(1),envelope:range(0,4)},
    ]),
    first:clip('first',0,2,media('movie','v0',0,2,2)),last:clip('last',2,4,media('movie','v0',4,2,2)),
    text:clip('text',0,4,{kind:'text',text:'中文 · Agent',font:'font',fontSizeMilliPixels:48000,rgba:[255,255,255,255]}),
    solid:clip('solid',1,3,{kind:'solid',rgba:[0,0,0,128]}),
    image:clip('image',0,4,media('image','still',0,0,4)),
    'audio-group':clip('audio-group',0,4,nested('child',4,0,4),[{kind:'gain',milliDecibels:-6000}]),
    narration:{...clip('narration',0,4,media('movie','a0',4,0,4)),audio:{...policy,rate:'resample'}},
    hold:clip('hold',0,4,media('movie','a0',1,1,4)),
    mute:{...clip('mute',0,4,media('movie','a0',4,0,4)),audio:{...policy,reverse:'mute'}},
  },
  resources:{movie,font:{id:'font',digest:'24'.repeat(32),byteLength:'100',descriptor:{kind:'font',faceIndex:0}},
    image:{id:'image',digest:'35'.repeat(32),byteLength:'100',descriptor:{kind:'media',streams:{still:{kind:'image',dimensions}}}}},
}
const binding = {resource:'movie',digest:movie.digest,stream:'v0',descriptor:movie.descriptor.streams.v0}
const window = {resourceDigest:movie.digest,stream:'v0',coverage:range(0,160,1000),samples:[
  {id:7,interval:range(0,33,1000)},{id:3,interval:range(33,75,1000)},{id:9,interval:range(100,160,1000)},
]}
const presentation = (value, changed={}) => ({type:'presentation',window,binding,sample:value,...changed})
const fixture = {document,queries:[
  {type:'frame',index:'0'},
  {type:'visualAt',sample:sample(2)},
  {type:'visualAt',sample:sample(4,1,'before')},
  {type:'visualAt',sample:sample(1,1000000001)},
  {type:'frame',index:'120'},
  {type:'audioSample',index:'0'},
  {type:'audioAt',sample:sample(2,1,'before')},
  {type:'audioSample',index:'192000'},
  presentation(sample(33,1000,'before')),
  presentation(sample(33,1000)),
  presentation(sample(80,1000)),
  presentation(sample(160,1000)),
  presentation(sample(160,1000,'before')),
  presentation(sample(0),{binding:{...binding,digest:'00'.repeat(32)}}),
  presentation(sample(0),{window:{...window,samples:[...window.samples].reverse()}}),
]}
const raw = JSON.stringify(fixture)
const native = spawnSync(resolve(nativePath), ['--projection'], {input:raw,encoding:'utf8',maxBuffer:32*1024*1024})
assert.equal(native.status,0,native.stderr)
const result = JSON.parse(native.stdout)
assert.equal(result.status,'complete',result.error)
assert.deepEqual(JSON.parse(wasm.video_projection_conformance(raw)),result)
assert.equal(result.frameCount,'120')
assert.equal(result.audioSampleCount,'192000')
const rejected = new Set([4,7,13,14])
assert.deepEqual(result.results.map(r=>r.status),fixture.queries.map((_,i)=>rejected.has(i)?'rejected':'computed'))
const first = result.results[0].value
assert.deepEqual(first.layers.map(l=>l.clip),['group','text','image'])
assert.equal(first.layers[0].effects[0].opacityPpm,500000)
assert.deepEqual(first.layers[0].effects[1].factor,ratio(0))
const child = first.layers[0].source.layers[0]
assert.equal(child.clip,'last')
assert.deepEqual(child.source.mapping.sample,sample(2)) // two reversals restore the right-side limit
assert.deepEqual(child.effects,[]) // parent opacity remains a group effect
assert.equal(first.layers[1].source.fontDigest,document.resources.font.digest)
assert.equal(first.layers[2].source.binding.descriptor.kind,'image')
assert.equal(result.results[1].value[0].source.layers[0].clip,'first')
assert.equal(result.results[3].value[0].source.layers[0].source.mapping.sample.time.denominator,'1000000001')
const audio = result.results[5].value.voices
assert.equal(audio.length,1) // hold and explicit reverse mute both contribute silence
assert.equal(audio[0].policy.rate,'preservePitch')
assert.equal(audio[0].source.voices[0].policy.rate,'resample')
assert.deepEqual(audio[0].source.voices[0].mapping.sample,sample(0))
assert.deepEqual(result.results.slice(8,13).map(r=>r.value.status),['sample','sample','gap','outsideWindow','sample'])
assert.deepEqual([8,9,12].map(i=>result.results[i].value.sample.id),[7,3,9])
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')
const report = {format:'musteroffice.video-projection-conformance/1',status:'pass',node:process.version,platform:process.platform,arch:process.arch,
  fixtureSha256:sha256(raw),resultSha256:sha256(native.stdout),queries:fixture.queries.length,computed:fixture.queries.length-rejected.size,rejected:rejected.size,
  nativeSha256:sha256(readFileSync(resolve(nativePath))),wasmBindingSha256:sha256(readFileSync(resolve(wasmPath))),
  wasmBinarySha256:sha256(readFileSync(resolve(wasmPath.replace(/\.js$/,'_bg.wasm')))),
  scope:'Exact projection, compositing hierarchy, audio point semantics and VFR presentation selection. Synthetic metadata only; not codec, DSP, GPU, media-quality or product acceptance.'}
writeFileSync(reportPath,`${JSON.stringify(report,null,2)}\n`)
console.log(JSON.stringify(report))
