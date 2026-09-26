import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {strokeFixtures} from './stroke-fixtures.mjs';
import {miterClipFixtures} from './miter-clip-fixtures.mjs';
export const U=1n<<32n;
export const point=(x=0n,y=0n)=>({x:String(x),y:String(y)});
export const base=()=>({viewport:{width:64,height:64,origin:point(),scale:{numerator:1,denominator:1},coordinateTolerance:String(1<<24),background:[0,0,0,0]},paths:[],draws:[]});
const rectangle=()=>({fillRule:'nonzero',commands:[{kind:'move',to:point(8n*U,8n*U)},{kind:'line',to:point(40n*U,8n*U)},{kind:'line',to:point(40n*U,40n*U)},{kind:'line',to:point(8n*U,40n*U)},{kind:'close'}]});
export function solid(){const q=base();q.paths=[rectangle()];q.draws=[{path:0,origin:point(),brush:{kind:'solid',rgba:[255,0,0,255]}}];return q;}
function fromFrame(bytes){
 const words=Uint32Array.from({length:bytes.length/4},(_,i)=>bytes.readUInt32LE(i*4)),q=base();
 q.viewport.width=words[2];q.viewport.height=words[3];q.viewport.background=rgba(words[4]);
 function rgba(w){return [w&255,w>>>8&255,w>>>16&255,w>>>24];}
 function p(){const a=[];for(let k=0;k<2;k++){const b=Buffer.alloc(4);b.writeUInt32LE(words[i++]);const v=b.readFloatLE()*Number(U);assert(Number.isSafeInteger(v));a.push(BigInt(v));}return point(...a);}
 let i=10;
 for(let j=0;j<words[5];j++){
  const fillRule=words[i++]?'evenodd':'nonzero',count=words[i++],commands=[];
  for(let k=0;k<count;k++){
   const start=i,op=words[i++];let c;
   if(op===1||op===2)c={kind:op===1?'move':'line',to:p()};
   if(op===3)c={kind:'quadratic',control:p(),to:p()};
   if(op===4)c={kind:'cubic',control1:p(),control2:p(),to:p()};
   if(op===5)c={kind:'close'};
   assert(c);commands.push(c);i=start+7;
  }
  q.paths.push({fillRule,commands});
 }
 assert.equal(words[1],4);assert.equal(words[8],0);assert.equal(words[9],0);
 for(let j=0;j<words[6];j++){
  q.draws.push({path:words[i++],origin:p(),brush:{kind:'solid',rgba:rgba(words[i++])}});assert.equal(words[i++],0);assert.equal(words[i++],0);
 }
 assert.equal(i,words.length);return q;
}
export function* fixtures(){
 yield* strokeFixtures();
 yield* miterClipFixtures();
 // Independently specified component shapes, without its previously rounded text.
 const old=JSON.parse(readFileSync('.codex-work/skia/verification/fixtures.json'));
 const names=['empty','background','transparent-background','solid','alpha-overlap','winding-solid','winding-hole','evenodd-hole','translated-clip','half-pixel','empty-path','move-only','open-contour','quad','cubic'];
 for(const name of names){const c=old.cases.find(c=>c.name===name);assert(c);yield {name:'shape-'+name,q:fromFrame(readFileSync(c.requestPath)),originalPixels:'.codex-work/skia/verification/'+name+'.rgba'};}
 for(const c of JSON.parse(readFileSync('.codex-work/paragraph-paths/parity.json')).cases){
  const r=JSON.parse(readFileSync(c.responsePath));if(r.status!=='evaluated'||!r.result.scene)continue;
  const scene=r.result.scene,q=base();q.viewport.width=q.viewport.height=512;q.viewport.background=[248,249,252,255];
  let span=1n,min=point();
  if(scene.bounds){min=scene.bounds.min;span=['x','y'].map(k=>BigInt(scene.bounds.max[k])-BigInt(min[k])).reduce((a,b)=>a>b?a:b,1n);}
  // 448 device pixels of content, at least 32 pixels of explicit margin.
  const denominator=(span+U-1n)/U,margin=(denominator*U+13n)/14n;assert(denominator<=0xffffffffn);
  q.viewport.scale={numerator:448,denominator:Number(denominator)};
  q.viewport.origin=point(BigInt(min.x)-margin,BigInt(min.y)-margin);
  q.paths=scene.paths.map(p=>({fillRule:'nonzero',commands:p.commands}));
  q.draws=scene.glyphs.map(g=>({path:g.path,origin:g.origin,brush:{kind:'solid',rgba:[28,42,60,255]}}));
  yield {name:'paragraph-'+c.name,q,paragraphSource:c.responsePath};
 }
 const shifted=solid(),huge=1n<<110n;
 shifted.viewport.origin=point(huge,-huge);shifted.draws[0].origin=point(huge,-huge);
 yield {name:'huge-draw-origin',q:shifted,equivalent:'plain-solid'};
 const anchored=solid();anchored.viewport.origin=point(huge,-huge);
 for(const c of anchored.paths[0].commands)if(c.to)c.to=point(BigInt(c.to.x)+huge,BigInt(c.to.y)-huge);
 yield {name:'huge-path-origin',q:anchored,equivalent:'plain-solid'};
 yield {name:'plain-solid',q:solid()};
 const alias=solid();alias.viewport.scale={numerator:0xffffffff,denominator:0xffffffff};yield {name:'reduced-scale',q:alias,equivalent:'plain-solid'};
 const tiny=base();tiny.paths=[{fillRule:'nonzero',commands:[{kind:'move',to:point()},{kind:'line',to:point(1n,-1n)}]}];tiny.draws=[{path:0,origin:point(),brush:{kind:'solid',rgba:[0,0,0,255]}}];tiny.viewport.scale.denominator=0xffffffff;yield {name:'minimum-rational',q:tiny};
 const mid=structuredClone(tiny);mid.paths[0].commands[1].to=point(((U+256n)*0xffffffffn)+1n,0n);yield {name:'double-rounding-counterexample',q:mid};
 const cancellation=solid();cancellation.viewport.origin=point((1n<<127n)-1n,0n);cancellation.draws[0].origin=structuredClone(cancellation.viewport.origin);yield {name:'i128-add-cancellation',q:cancellation,equivalent:'plain-solid'};
 let state=0x534b4941;function rng(){state=(Math.imul(state,1664525)+1013904223)>>>0;return state;}
 for(let i=0;i<32;i++){
  const q=base(),n=1+rng()%0x7fffffff,d=1+rng()%0xffffffff;
  q.viewport.scale={numerator:n,denominator:d};
  // Arbitrary rational control points around both signs and exponent boundaries.
  function coord(){return BigInt((rng()%8192)-4096)*BigInt(d)*U/(16n*BigInt(n))+BigInt(rng()%513)-256n;}
  const commands=[{kind:'move',to:point()}];
  for(let k=0;k<12;k++)commands.push({kind:'cubic',control1:point(coord(),coord()),control2:point(coord(),coord()),to:point(coord(),coord())});
  commands.push({kind:'close'});q.paths=[{fillRule:i%2?'evenodd':'nonzero',commands}];q.draws=[{path:0,origin:point(coord()/8n,coord()/8n),brush:{kind:'solid',rgba:[rng()%256,rng()%256,rng()%256,128]}}];
  yield {name:'rational-'+i,q};
 }
 const boundary=base();boundary.paths=[{fillRule:'nonzero',commands:[{kind:'move',to:point()},{kind:'line',to:point(32768n*U,-32768n*U)}]}];boundary.draws=[{path:0,origin:point(),brush:{kind:'solid',rgba:[1,2,3,255]}}];yield {name:'coordinate-boundary',q:boundary};
 const width=base();width.viewport.width=8192;width.viewport.height=1;yield {name:'maximum-width',q:width};
 const paths=base();paths.paths=Array.from({length:4096},()=>({fillRule:'nonzero',commands:[]}));yield {name:'maximum-paths',q:paths};
 const draws=base();draws.paths=[{fillRule:'nonzero',commands:[]}];draws.draws=Array.from({length:65536},()=>({path:0,origin:point(),brush:{kind:'solid',rgba:[0,0,0,0]}}));yield {name:'maximum-draws',q:draws};
 const commands=base();commands.paths=[{fillRule:'nonzero',commands:Array.from({length:262144},()=>({kind:'move',to:point()}))}];yield {name:'maximum-commands',q:commands};
 const work=base();work.paths=[{fillRule:'nonzero',commands:commands.paths[0].commands.slice(0,1024)}];work.draws=draws.draws.slice(0,1024);yield {name:'maximum-drawn-commands',q:work};
 function bad(name,change,expected='INPUT_INVALID',validRequest=true){const q=solid();change(q);return {name,q,expected,validRequest};}
 yield bad('zero-width',q=>q.viewport.width=0);
 yield bad('over-width',q=>q.viewport.width=8193,'LIMIT_EXCEEDED');
 yield bad('over-pixels',q=>{q.viewport.width=8192;q.viewport.height=4096;},'LIMIT_EXCEEDED');
 yield bad('zero-numerator',q=>q.viewport.scale.numerator=0);
 yield bad('zero-denominator',q=>q.viewport.scale.denominator=0);
 yield bad('small-tolerance',q=>q.viewport.coordinateTolerance='255');
 yield bad('large-tolerance',q=>q.viewport.coordinateTolerance='16777217');
 yield bad('noncanonical-fixed',q=>q.viewport.origin.x='00','INPUT_INVALID',false);
 yield bad('overflow-fixed',q=>q.viewport.origin.x=String(1n<<127n),'INPUT_INVALID',false);
 yield bad('numeric-fixed',q=>q.viewport.origin.x=0,'INPUT_INVALID',false);
 yield bad('unknown-field',q=>q.viewport.implicitFont=true,'INPUT_INVALID',false);
 yield bad('invalid-color',q=>q.draws[0].brush.rgba[0]=256,'INPUT_INVALID',false);
 yield bad('missing-reference',q=>q.draws[0].path=1);
 yield bad('first-not-move',q=>q.paths[0].commands.shift());
 yield bad('after-close',q=>q.paths[0].commands.push({kind:'line',to:point()}));
 yield bad('double-close',q=>q.paths[0].commands.push({kind:'close'}));
 yield bad('local-range',q=>q.paths[0].commands[1].to.x=String(40000n*U),'COORDINATE_RANGE');
 yield bad('offset-range',q=>q.draws[0].origin.x=String(40000n*U),'COORDINATE_RANGE');
 yield bad('combined-range',q=>q.draws[0].origin.x=String(32760n*U),'COORDINATE_RANGE');
 yield bad('combined-i128-overflow',q=>{q.viewport.origin.x=String(-(1n<<127n));q.draws[0].origin.x=String((1n<<127n)-1n);},'COORDINATE_RANGE');
 yield bad('precision-rejected',q=>{q.viewport.coordinateTolerance='256';q.viewport.scale={numerator:1,denominator:3};q.draws[0].origin.x='1';},'PRECISION_EXCEEDED');
 for(const [name,q] of [['paths',paths],['draws',draws],['commands',commands],['drawn-commands',work]]){
  const t=structuredClone(q);
  if(name==='paths')t.paths.push({fillRule:'nonzero',commands:[]});
  if(name==='draws'||name==='drawn-commands')t.draws.push({path:0,origin:point(),brush:{kind:'solid',rgba:[0,0,0,0]}});
  if(name==='commands')t.paths[0].commands.push({kind:'move',to:point()});
  yield {name:'over-'+name,q:t,expected:'LIMIT_EXCEEDED'};
 }
 yield {name:'duplicate-key',q:JSON.stringify(solid()).replace('"width":64','"width":64,"width":64'),expected:'INPUT_INVALID',validRequest:false};
 yield {name:'truncated-json',q:'{"viewport":',expected:'INPUT_INVALID',validRequest:false};
}
