// Owned geometry and failure probes. The pixel oracle is independent Python geometry.
const U=1n<<32n;
const p=(x,y)=>({x:String(BigInt(x)*U),y:String(BigInt(y)*U)});
function request(cap='butt',join={kind:'round'},width=8n*U){
 return {viewport:{width:64,height:64,origin:p(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:String(1<<24),background:[0,0,0,0]},
  paths:[{fillRule:'nonzero',commands:[{kind:'move',to:p(20,32)},{kind:'line',to:p(44,32)}]}],
  draws:[{path:0,origin:p(0,0),brush:{kind:'solid',rgba:[255,0,0,255]},stroke:{width:String(width),cap,join}}]};
}
export function* strokeFixtures(){
 for(const cap of ['butt','round','square']){
  yield {name:'stroke-cap-'+cap,q:request(cap)};
  const zero=request(cap);zero.paths[0].commands[1].to=p(20,32);
  yield {name:'stroke-zero-segment-'+cap,q:zero};
  for(const width of [0n,1n,U/4n,U])yield {name:`stroke-thin-${cap}-${width}`,q:request(cap,{kind:'round'},width)};
 }
 for(const [name,join] of [['round',{kind:'round'}],['bevel',{kind:'bevel'}],['miter',{kind:'miter',limit:String(4n*U)}],['miter-clipped',{kind:'miter',limit:String(U)}],['miter-zero',{kind:'miter',limit:'0'}]]){
  const q=request('butt',join,16n*U);q.paths[0].commands=[{kind:'move',to:p(20,44)},{kind:'line',to:p(20,20)},{kind:'line',to:p(44,20)}];
  yield {name:'stroke-join-'+name,q};
 }
 for(const closed of [false,true]){
  const q=request();q.paths[0].commands=[{kind:'move',to:p(12,12)},{kind:'line',to:p(52,12)},{kind:'line',to:p(52,52)},{kind:'line',to:p(12,52)},...(closed?[{kind:'close'}]:[])];
  yield {name:'stroke-contour-'+(closed?'closed':'open'),q};
 }
 const both=request();both.paths[0].commands=[{kind:'move',to:p(12,12)},{kind:'line',to:p(52,12)},{kind:'line',to:p(52,52)},{kind:'line',to:p(12,52)},{kind:'close'}];
 both.draws.unshift({path:0,origin:p(0,0),brush:{kind:'solid',rgba:[0,0,255,255]}});
 both.draws.push({path:0,origin:p(24,24),brush:{kind:'solid',rgba:[0,255,0,128]}});
 yield {name:'stroke-fill-order-reset',q:both};
 const shared=request('round');shared.draws.push({...structuredClone(shared.draws[0]),origin:p(0,16)});
 yield {name:'stroke-style-sharing',q:shared};
 const expanded=request('round');expanded.paths[0].commands[0].to=p(8,32);expanded.paths[0].commands[1].to=p(56,32);
 yield {name:'stroke-expanded-line',q:expanded};
 const shifted=request('square');const huge=1n<<110n;shifted.viewport.origin.x=String(huge);shifted.draws[0].origin.x=String(huge);
 yield {name:'stroke-huge-origin',q:shifted,equivalent:'stroke-cap-square'};
 const scaled=request('square', {kind:'round'},16n*U);scaled.viewport.scale.denominator=2;
 for(const c of scaled.paths[0].commands)for(const a of ['x','y'])c.to[a]=String(BigInt(c.to[a])*2n);
 yield {name:'stroke-viewport-scale',q:scaled,equivalent:'stroke-cap-square'};
 const rounding=request();rounding.viewport.scale.denominator=0xffffffff;rounding.draws[0].stroke.width=String((U+256n)*0xffffffffn+1n);
 yield {name:'stroke-width-double-rounding',q:rounding};
 const miterRounding=request('square',{kind:'miter',limit:String(4n*U+1n)});
 yield {name:'stroke-miter-quantization',q:miterRounding};
 for(let i=0;i<32;i++){
  const q=request(['butt','round','square'][i%3],i%2?{kind:'bevel'}:{kind:'miter',limit:String((2n+BigInt(i))*U+127n)},(2n+BigInt(i))*U+BigInt(i));
  q.viewport.scale={numerator:7,denominator:13};
  q.paths[0].commands=[{kind:'move',to:p(8,32)},{kind:'quadratic',control:p(32,-i),to:p(56,32)},{kind:'cubic',control1:p(56,48+i),control2:p(8,48),to:p(8,32)}];
  if(i%2)q.paths[0].commands.push({kind:'close'});
  q.draws[0].brush.rgba=[i*7,255-i*7,i,160];
  yield {name:'stroke-curve-'+i,q};
 }
 const styles=request();styles.paths[0].commands=[];styles.draws=Array.from({length:4096},(_,i)=>({...structuredClone(styles.draws[0]),stroke:{width:String(BigInt(i)*U),cap:'butt',join:{kind:'round'}}}));
 yield {name:'stroke-maximum-styles',q:styles};
 const over=structuredClone(styles);over.draws.push({...structuredClone(over.draws[0]),stroke:{width:'0',cap:'round',join:{kind:'round'}}});
 yield {name:'stroke-over-styles',q:over,expected:'LIMIT_EXCEEDED'};
 const invalid=[
  ['negative-width',q=>q.draws[0].stroke.width='-1','INPUT_INVALID'],
  ['width-range',q=>q.draws[0].stroke.width=String(32768n*U+1n),'COORDINATE_RANGE'],
  ['negative-miter',q=>q.draws[0].stroke.join={kind:'miter',limit:'-1'},'INPUT_INVALID'],
  ['miter-range',q=>q.draws[0].stroke.join={kind:'miter',limit:String(1024n*U+1n)},'INPUT_INVALID'],
  ['expanded-range',q=>q.draws[0].origin=p(32722,0),'COORDINATE_RANGE'],
  ['hairline-range',q=>{q.draws[0].stroke.width='0';q.draws[0].origin=p(32724,0);},'COORDINATE_RANGE'],
  ['width-precision',q=>{q.viewport.scale.denominator=3;q.viewport.coordinateTolerance='256';q.draws[0].stroke.width=String(1000n*U);},'PRECISION_EXCEEDED'],
 ];
 for(const [name,change,expected] of invalid){const q=request();change(q);yield {name:'stroke-reject-'+name,q,expected};}
 for(const [name,change] of [
  ['cap',q=>q.draws[0].stroke.cap='flat'],['join',q=>q.draws[0].stroke.join={kind:'round',limit:'0'}],
  ['missing-width',q=>delete q.draws[0].stroke.width],['width-number',q=>q.draws[0].stroke.width=8],
 ]){const q=request();change(q);yield {name:'stroke-reject-'+name,q,expected:'INPUT_INVALID',validRequest:false};}
}
