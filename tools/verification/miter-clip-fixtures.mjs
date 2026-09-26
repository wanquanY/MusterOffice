// Owned inputs. Expected ink is computed independently by polygon intersection
// in miter-clip-reference.py, never by replaying the component join callback.
const U=1n<<32n;
const point=(x,y)=>({x:String(BigInt(Math.round(x*65536))*65536n),y:String(BigInt(Math.round(y*65536))*65536n)});
function request(spread=16,width=24,limit=2){
 return {viewport:{width:192,height:192,origin:point(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:String(1<<24),background:[0,0,0,0]},
  paths:[{fillRule:'nonzero',commands:[{kind:'move',to:point(96-spread,156)},{kind:'line',to:point(96,64)},{kind:'line',to:point(96+spread,156)}]}],
  draws:[{path:0,origin:point(0,0),brush:{kind:'solid',rgba:[255,0,0,255]},stroke:{width:String(BigInt(width)*U),cap:'butt',join:{kind:'miterClip',limit:String(BigInt(limit*4)*U/4n)}}}]};
}
export function* miterClipFixtures(){
 for(const spread of [4,16,40])for(const width of [8,24])for(const limit of [1,1.25,2,4,16])
  yield {name:`clip-v-${spread}-${width}-${limit}`,q:request(spread,width,limit)};
 for(const reversed of [false,true])for(let rotation=0;rotation<4;rotation++){
  const q=request();let pts=q.paths[0].commands.map(c=>[Number(BigInt(c.to.x)/U),Number(BigInt(c.to.y)/U)]);
  if(reversed)pts.reverse();
  for(let j=0;j<rotation;j++)pts=pts.map(([x,y])=>[192-y,x]);
  q.paths[0].commands=pts.map(([x,y],i)=>({kind:i?'line':'move',to:point(x,y)}));
  yield {name:`clip-direction-${Number(reversed)}-${rotation}`,q};
 }
 for(const [name,pts,closed] of [
  ['rectangle',[[48,48],[144,48],[144,144],[48,144]],true],
  ['rectangle-reverse',[[48,48],[48,144],[144,144],[144,48]],true],
  ['short',[[94,72],[96,64],[98,72]],false],
  ['straight',[[32,96],[96,96],[160,96]],false],
  ['reversal',[[32,96],[144,96],[32,96]],false],
  ['almost-reversal',[[32,96],[144,96],[32,96+1/65536]],false],
  ['almost-straight',[[32,96],[96,96],[160,96+1/65536]],false],
  ['duplicate',[[80,156],[96,64],[96,64],[112,156]],false],
  ['move-only',[[96,96]],false],
  ['zero-segment',[[96,96],[96,96]],false],
 ]){
  const q=request(16,24,1.25);q.paths[0].commands=pts.map(([x,y],i)=>({kind:i?'line':'move',to:point(x,y)}));
  if(closed)q.paths[0].commands.push({kind:'close'});
  yield {name:'clip-'+name,q};
 }
 for(const cap of ['butt','round','square']){
  const q=request();q.draws[0].stroke.cap=cap;yield {name:'clip-cap-'+cap,q};
  const hair=structuredClone(q);hair.draws[0].stroke.width='0';yield {name:'clip-hairline-'+cap,q:hair};
 }
 for(const width of [1n,U/4n,U,32768n*U]){
  const q=request();q.draws[0].stroke.width=String(width);
  yield {name:'clip-width-'+width,q, ...(width>100n*U?{expected:'COORDINATE_RANGE'}:{})};
 }
 for(const kind of ['quadratic','cubic'])for(const flip of [false,true]){
  const q=request();q.paths[0].commands=[{kind:'move',to:point(16,160)},
   ...(kind==='quadratic'?[{kind,control:point(80,156),to:point(96,64)},{kind,control:point(112,156),to:point(176,160)}]:
    [{kind,control1:point(24,32),control2:point(80,156),to:point(96,64)},{kind,control1:point(112,156),control2:point(168,32),to:point(176,160)}])];
  if(flip)for(const c of q.paths[0].commands)for(const key of ['to','control','control1','control2'])if(c[key])c[key].x=String(192n*U-BigInt(c[key].x));
  yield {name:`clip-${kind}-${Number(flip)}`,q};
 }
 const alpha=request();alpha.draws[0].brush.rgba=[255,0,0,128];yield {name:'clip-alpha',q:alpha};
 const huge=request();huge.viewport.origin.x=String(1n<<110n);huge.draws[0].origin.x=huge.viewport.origin.x;
 yield {name:'clip-huge-origin',q:huge,equivalent:'clip-v-16-24-2'};
 const scaled=request();scaled.viewport.scale.denominator=2;
 for(const c of scaled.paths[0].commands)for(const k of ['x','y'])c.to[k]=String(BigInt(c.to[k])*2n);
 scaled.draws[0].stroke.width=String(48n*U);
 yield {name:'clip-scale',q:scaled,equivalent:'clip-v-16-24-2'};
 const quantized=request();quantized.draws[0].stroke.join.limit=String(2n*U+1n);yield {name:'clip-quantization',q:quantized};
 for(const limit of ['-1','0',String(U-1n),String(1024n*U+1n)]){
  const q=request();q.draws[0].stroke.join.limit=limit;yield {name:'clip-reject-limit-'+limit,q,expected:'INPUT_INVALID'};
 }
 for(const width of ['0',String(24n*U)]){
  const q=request();q.draws[0].stroke.width=width;q.draws[0].stroke.join.limit=String(1024n*U);q.draws[0].origin=point(32700,0);
  yield {name:'clip-reject-expanded-'+width,q,expected:'COORDINATE_RANGE'};
 }
 for(const [name,change] of [['missing',q=>delete q.draws[0].stroke.join.limit],['extra',q=>q.draws[0].stroke.join.clipPixels=5]]){
  const q=request();change(q);yield {name:'clip-reject-'+name,q,expected:'INPUT_INVALID',validRequest:false};
 }
}
