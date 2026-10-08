// Shared test fixture; never used as a production media provider.
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
export const fixture={document,pcm:[{resource:'sound',stream:'a0',sampleZero:ratio(0),firstSample:'0',samples:pcm}],queries:[{first:'0',frames:2048},{first:'123',frames:137},{first:'260',frames:1000},{first:'2000',frames:96},{first:'2500',frames:50},{first:'3072',frames:100},{first:'4096',frames:1},{first:'-1',frames:10}]}
