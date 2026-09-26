import {base,point,U} from './path-raster-fixtures.mjs';
export function request(){
 const q=base();q.paths=[{fillRule:'nonzero',commands:[
  {kind:'move',to:point()},{kind:'line',to:point(64n*U,0n)},
  {kind:'line',to:point(64n*U,64n*U)},{kind:'line',to:point(0n,64n*U)},{kind:'close'},
 ]}];
 q.draws=[{path:0,origin:point(),brush:{kind:'gradient',gradient:{
  geometry:{kind:'linear',start:point(16n*U,8n*U),end:point(48n*U,32n*U)},
  stops:[{position:0,srgb:[1,0.15,0.1,0.2]},{position:0.375,srgb:[0.1,0.9,0.3,0.8]},
   {position:0.75,srgb:[0.2,0.1,1,0.4]},{position:1,srgb:[1,0.8,0.1,1]}],
  tile:'clamp',interpolation:'srgb',alpha:'straight',
 }}}];return q;
}
const grad=q=>q.draws[0].brush.gradient;
export function* fixtures(){
 for(const kind of ['linear','radial'])for(const tile of ['clamp','repeat','mirror','decal'])
  for(const interpolation of ['srgb','linearSrgb'])for(const alpha of ['straight','premultiplied']){
   const q=request(),g=grad(q);if(kind==='radial')g.geometry={kind,center:point(32n*U,32n*U),radius:String(20n*U)};
   Object.assign(g,{tile,interpolation,alpha});
   yield {name:[kind,tile,interpolation,alpha].join('-'),q,oracle:true};
  }
 for(const [name,edit] of [
  ['hard-stops',g=>{g.geometry={kind:'linear',start:point(),end:point(64n*U,0n)};g.stops=[
    {position:0,srgb:[1,0,0,1]},{position:16.5/64,srgb:[1,0,0,1]},
    {position:16.5/64,srgb:[0,0,1,1]},{position:1,srgb:[0,0,1,1]}];}],
  ['implicit-endpoints',g=>{g.stops[0].position=0.125;g.stops.at(-1).position=0.875;}],
  ['sub-rgba8',g=>g.stops.forEach((s,i)=>s.srgb=[(i+0.25)/255,0,0,1])],
  ['extended-channels',g=>g.stops.forEach((s,i)=>s.srgb=[i/2,-0.1,1.1,1])],
  ['transparent-colors',g=>{g.stops[0].srgb=[1,0,0,0];g.stops[1].srgb=[0,0,1,1];g.stops=g.stops.slice(0,2);g.stops[1].position=1;}],
 ]){const q=request();edit(grad(q));yield {name,q,oracle:true};}
 const shifted=request(),huge=1n<<110n;
 shifted.viewport.origin=point(huge,-huge);shifted.draws[0].origin=point(huge,-huge);
 for(const key of ['start','end']){const p=grad(shifted).geometry[key];p.x=String(BigInt(p.x)+huge);p.y=String(BigInt(p.y)-huge);}
 yield {name:'huge-world-origin',q:shifted,equivalent:'linear-clamp-srgb-straight',oracle:true};
 const translated=request();translated.draws[0].origin=point(8n*U,4n*U);
 yield {name:'path-translation-world-brush',q:translated,oracle:true};
 const duplicate=request();duplicate.draws[0].brush.gradient.stops.forEach(s=>s.srgb[3]=1);
 duplicate.draws.push({...structuredClone(duplicate.draws[0]),stroke:{width:String(U*2n),cap:'round',join:{kind:'miterClip',limit:String(U*4n)}}});
 duplicate.draws.push({path:0,origin:point(48n*U,48n*U),brush:{kind:'solid',rgba:[255,0,0,255]}});
 yield {name:'gradient-stroke-solid-state',q:duplicate};
 for(const [name,edit,expected='INPUT_INVALID'] of [
  ['missing-stops',g=>g.stops=[]],['one-stop',g=>g.stops=g.stops.slice(0,1)],
  ['descending-stops',g=>g.stops.reverse()],['negative-stop',g=>g.stops[0].position=-1],
  ['alpha-overflow',g=>g.stops[0].srgb[3]=1.001],['color-range',g=>g.stops[0].srgb[0]=65505,'INPUT_INVALID'],
  ['collapsed-stops',g=>{g.stops[0].position=0.25;g.stops[1].position=0.25+1e-10;},'PRECISION_EXCEEDED'],
  ['collapsed-endpoint',g=>g.stops.at(-1).position=1-1e-10,'PRECISION_EXCEEDED'],
  ['collapsed-axis',g=>g.geometry.end=g.geometry.start,'PRECISION_EXCEEDED'],
  ['zero-radius',g=>g.geometry={kind:'radial',center:point(),radius:'0'}],
  ['tiny-radius',g=>g.geometry={kind:'radial',center:point(),radius:'1'},'PRECISION_EXCEEDED'],
  ['huge-geometry',g=>g.geometry.end=point(32769n*U,0n),'COORDINATE_RANGE'],
 ]){const q=request();edit(grad(q));yield {name:'reject-'+name,q,expected};}
 for(const [name,edit] of [
  ['legacy-color',q=>{q.draws[0].color=[255,0,0,255];delete q.draws[0].brush;}],
  ['ambiguous-solid',q=>q.draws[0].brush={kind:'solid',rgba:[0,0,0,0],gradient:grad(q)}],
  ['unknown-interpolation',q=>grad(q).interpolation='browser-default'],
  ['radial-extra',q=>grad(q).geometry={kind:'radial',center:point(),radius:String(U),end:point()}],
 ]){const q=request();edit(q);yield {name:'reject-'+name,q,expected:'INPUT_INVALID',validRequest:false};}
 // The shader is already in world space. A transformed reused path must not
 // transform its paint a second time; both paths cover exactly this viewport.
 const scene=request();scene.scene={paths:scene.paths,transforms:[{parent:null,affine:{linear:[String(U),String(U/2n),'0',String(U)],translation:point(-32n*U,0n)}}],
  instances:[{path:0,transform:0,brush:scene.draws[0].brush}]};delete scene.paths;delete scene.draws;
 yield {name:'scene-shear-world-brush',q:scene,scene:true};
 const sceneHuge=structuredClone(scene);sceneHuge.viewport.origin=point(huge,-huge);
 sceneHuge.scene.transforms[0].affine.translation=point(huge-32n*U,-huge);
 for(const key of ['start','end']){const p=sceneHuge.scene.instances[0].brush.gradient.geometry[key];p.x=String(BigInt(p.x)+huge);p.y=String(BigInt(p.y)-huge);}
 yield {name:'scene-huge-world-origin',q:sceneHuge,scene:true,equivalent:'scene-shear-world-brush'};
}
