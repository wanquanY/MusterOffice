import {readFileSync} from 'node:fs';
import {base,solid,point,U} from './path-raster-fixtures.mjs';
import {strokeFixtures} from './stroke-fixtures.mjs';
export const identity=[U,0n,0n,U].map(String);
export function node(parent=null,linear=identity,translation=point()){return {parent,affine:{linear:[...linear],translation:{...translation}}};}
export function fromPaths(q){
 const scene={paths:q.paths,transforms:[],instances:[]},origins=new Map();
 for(const d of q.draws){
  const key=JSON.stringify(d.origin);let transform=origins.get(key);
  if(transform===undefined){transform=scene.transforms.length;scene.transforms.push(node(null,identity,d.origin));origins.set(key,transform);}
  scene.instances.push({path:d.path,transform,brush:d.brush,...(d.stroke?{stroke:d.stroke}:{})});
 }
 return {viewport:q.viewport,scene};
}
const q32=(n,d=1n)=>String(BigInt(n)*U/BigInt(d));
function round(n){const sign=n<0n?-1n:1n;return sign*((n*sign+U/2n)/U);}
export function* fixtures(){
 const stroke=fromPaths([...strokeFixtures()].find(c=>c.name==='stroke-cap-round').q);
 for(const [name,linear,translation,reference] of [
  ['nonuniform',[2n*U,0n,0n,3n*U],point(-32n*U,-64n*U),'stroke-expanded-line'],
  ['reflection',[-U,0n,0n,U],point(64n*U,0n),'stroke-cap-round'],
  ['quarter-turn',[0n,-U,U,0n],point(64n*U,0n),null],
  ['shear',[U,U/3n,U/4n,U],point(),null],
  ['singular',[0n,0n,0n,0n],point(20n*U,32n*U),'stroke-zero-segment-round'],
 ]){
  const q=structuredClone(stroke);q.scene.transforms=[node(null,linear.map(String),translation)];
  yield {name:'world-stroke-'+name,q,...(reference?{referencePixels:'.codex-work/path-raster/'+reference+'.rgba'}:{})};
 }
 const prior=JSON.parse(readFileSync('.codex-work/path-raster/parity.json'));
 for(const c of prior.cases){
  if(c.status!=='rendered')continue;
  const q=fromPaths(JSON.parse(readFileSync(c.requestPath)));
  yield {name:'identity-'+c.name,q,referencePixels:c.pixelsPath};
  if(c.paragraphSource){
   // Rotate a complete paragraph around the explicit viewport centre, preserving
   // glyph resources and all local placement nodes under a shared group.
   const r=structuredClone(q),v=r.viewport;
   const cx=BigInt(v.origin.x)+256n*BigInt(v.scale.denominator)*U/BigInt(v.scale.numerator);
   const cy=BigInt(v.origin.y)+256n*BigInt(v.scale.denominator)*U/BigInt(v.scale.numerator);
   r.scene.transforms=r.scene.transforms.map(t=>({...t,parent:0}));
   r.scene.transforms.unshift(node(null,['0',String(-U),String(U),'0'],point(cx+cy,cy-cx)));
   r.scene.instances.forEach(i=>i.transform++);
   yield {name:'rotate-'+c.name,q:r,paragraphSource:c.paragraphSource};
  }
 }
 const shared=fromPaths(solid());shared.scene.transforms=[node(null,[q32(3n,4n),q32(1n,4n),q32(-1n,4n),q32(3n,4n)],point(4n*U,16n*U))];
 shared.scene.instances=[];
 for(let i=0;i<8;i++){shared.scene.transforms.push(node(0,identity,point(BigInt(i)*U/2n,0n)));shared.scene.instances.push({path:0,transform:i+1,brush:{kind:'solid',rgba:[20*i,20,255-20*i,32]}});}
 yield {name:'shared-shear-alpha',q:shared};
 for(const [name,linear,t] of [['horizontal-flip',[-U,0n,0n,U],point(64n*U,0n)],['vertical-flip',[U,0n,0n,-U],point(0n,64n*U)],['singular',[0n,0n,0n,U],point(0n,0n)],['nonuniform',[U/2n,U/4n,0n,U*3n/4n],point(8n*U,2n*U)]]){
  const q=fromPaths(solid());q.scene.transforms=[node(null,linear.map(String),t)];yield {name,q};
 }
 const huge=fromPaths(solid()),h=1n<<110n;
 for(const c of huge.scene.paths[0].commands)if(c.to)c.to=point(BigInt(c.to.x)+h,BigInt(c.to.y)-h);
 huge.scene.transforms=[node(null,identity,point(-h,h))];yield {name:'huge-source-cancellation',q:huge,referencePixels:'.codex-work/path-raster/plain-solid.rgba'};
 const minimum=fromPaths(solid()),min=-(1n<<127n);minimum.viewport.origin=point(min,0n);minimum.scene.transforms=[];minimum.scene.instances[0].transform=null;
 for(const c of minimum.scene.paths[0].commands)if(c.to)c.to=point(BigInt(c.to.x)+min,BigInt(c.to.y));
 yield {name:'minimum-viewport-origin',q:minimum,referencePixels:'.codex-work/path-raster/plain-solid.rgba',attempts:2};
 const unused=fromPaths(solid());unused.scene.transforms.push(node(null,[String(1n<<120n),'0','0',String(U)]),node(1,[String(1n<<120n),'0','0',String(U)]));
 yield {name:'unused-transform-range',q:unused,referencePixels:'.codex-work/path-raster/plain-solid.rgba',attempts:1};
 const empty=structuredClone(unused);empty.scene.paths.push({fillRule:'nonzero',commands:[]});empty.scene.instances.push({path:1,transform:2,brush:{kind:'solid',rgba:[0,0,0,0]}});
 yield {name:'empty-instance-transform-range',q:empty,referencePixels:'.codex-work/path-raster/plain-solid.rgba',attempts:1};
 const wide=fromPaths(solid());wide.scene.paths[0].commands=[{kind:'move',to:point(h,h)},{kind:'line',to:point(h+U,h+U)}];wide.scene.transforms=[node(null,[String(1n<<100n),String(-(1n<<100n)+U),'0',String(U)],point(-h,-h))];yield {name:'wide-products-cancellation',q:wide};
 const fallback=fromPaths(solid());fallback.scene.paths[0].commands=[{kind:'move',to:point()},{kind:'line',to:point(0n,U)}];fallback.scene.transforms=[node(null,[String(1n<<120n),'0','0',String(U)]),node(0,[String(1n<<120n),'0','0',String(U)])];fallback.scene.instances[0].transform=1;yield {name:'matrix-range-fallback',q:fallback,attempts:2};
 const precision=fromPaths(solid()),x=21000n*U,inner=U/7n,outer=U/3n,tx=1000n*U-round(outer*round(inner*x));
 precision.viewport.width=1024;precision.viewport.height=4;precision.viewport.coordinateTolerance='256';
 precision.scene.transforms=[node(null,[String(outer),'0','0',String(U)],point(tx,0n)),node(0,[String(inner),'0','0',String(U)])];precision.scene.instances[0].transform=1;
 precision.scene.paths[0].commands=[{kind:'move',to:point()},{kind:'line',to:point(x,0n)},{kind:'line',to:point(x,U)},{kind:'line',to:point(0n,U)},{kind:'close'}];yield {name:'matrix-precision-fallback',q:precision,attempts:2};
 let state=0x61666669;function rng(){state=(Math.imul(state,1664525)+1013904223)>>>0;return state;}
 for(let i=0;i<32;i++){
  const q=fromPaths(solid());q.scene.transforms=[];
  for(let j=0;j<8;j++)q.scene.transforms.push(node(j===0?null:j-1,[q32(900n+BigInt(rng()%201),1000n),q32(BigInt(rng()%201)-100n,1000n),q32(BigInt(rng()%201)-100n,1000n),q32(900n+BigInt(rng()%201),1000n)],point(BigInt(rng()%201)-100n,BigInt(rng()%201)-100n)));
  q.scene.instances[0].transform=7;yield {name:'nested-rational-'+i,q};
 }
 const nodes=fromPaths(base());nodes.scene.transforms=Array.from({length:8192},()=>node());yield {name:'maximum-nodes',q:nodes};
 const depth=fromPaths(solid());depth.scene.transforms=Array.from({length:64},(_,i)=>node(i===0?null:i-1));depth.scene.instances[0].transform=63;yield {name:'maximum-depth',q:depth};
 const work=structuredClone(depth);work.scene.paths[0].commands=[{kind:'move',to:point()},...Array.from({length:341},()=>({kind:'cubic',control1:point(),control2:point(),to:point()}))];
 work.scene.instances=Array.from({length:64},()=>({path:0,transform:63,brush:{kind:'solid',rgba:[0,0,0,0]}}));yield {name:'maximum-transform-work',q:work};
 function bad(name,change,expected='INPUT_INVALID',validRequest=true){const q=fromPaths(solid());change(q);return {name,q,expected,validRequest};}
 yield bad('self-parent',q=>q.scene.transforms[0].parent=0);
 yield bad('future-parent',q=>q.scene.transforms[0].parent=1);
 yield bad('missing-transform',q=>q.scene.instances[0].transform=10);
 yield bad('missing-path',q=>q.scene.instances[0].path=1);
 yield bad('unused-invalid-path',q=>q.scene.paths.push({fillRule:'nonzero',commands:[{kind:'close'}]}));
 yield bad('noncanonical-coefficient',q=>q.scene.transforms[0].affine.linear[0]='00','INPUT_INVALID',false);
 yield bad('numeric-coefficient',q=>q.scene.transforms[0].affine.linear[0]=1,'INPUT_INVALID',false);
 yield bad('coefficient-overflow',q=>q.scene.transforms[0].affine.linear[0]=String(1n<<127n),'INPUT_INVALID',false);
 yield bad('wrong-matrix-length',q=>q.scene.transforms[0].affine.linear.pop(),'INPUT_INVALID',false);
 yield bad('unknown-property',q=>q.scene.transforms[0].implicitMatrix=true,'INPUT_INVALID',false);
 yield bad('world-range',q=>q.scene.transforms[0].affine.translation.x=String(1n<<120n),'COORDINATE_RANGE');
 yield bad('unrepresentable-result',q=>q.scene.transforms[0].affine.linear[0]=String(1n<<120n),'COORDINATE_RANGE');
 for(const [name,q] of [['nodes',nodes],['depth',depth],['transform-work',work]]){
  const r=structuredClone(q);if(name==='nodes')r.scene.transforms.push(node());if(name==='depth')r.scene.transforms.push(node(63));if(name==='transform-work')r.scene.instances.push(structuredClone(r.scene.instances[0]));yield {name:'over-'+name,q:r,expected:'LIMIT_EXCEEDED'};
 }
 yield {name:'duplicate-key',q:JSON.stringify(fromPaths(solid())).replace('"width":64','"width":64,"width":64'),expected:'INPUT_INVALID',validRequest:false};
}
