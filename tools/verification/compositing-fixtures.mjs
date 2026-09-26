/** Evaluated Q32 probes. Snapshot paint remains device-aligned at huge origins. */
const U=1n<<32n;
export const fixed=n=>(BigInt(n)*U).toString();
export const point=(x,y)=>({x:fixed(x),y:fixed(y)});
const polygon=points=>({fillRule:'nonzero',commands:[...points.map(([x,y],i)=>({kind:i?'line':'move',to:point(x,y)})),{kind:'close'}]});
const rect=(l,t,r,b)=>polygon([[l,t],[r,t],[r,b],[l,b]]);
export function scene(q){
 const shift=q.viewport.origin;
 return {viewport:q.viewport,scene:{paths:q.paths,transforms:q.draws.map(d=>({parent:null,affine:{linear:[fixed(1),'0','0',fixed(1)],translation:d.origin}})),
  clips:q.clips.map(({origin,...v})=>({...v,transform:q.draws.findIndex(d=>d.origin.x===origin.x&&d.origin.y===origin.y)})),
  instances:q.draws.map(({origin,...d},i)=>({...d,transform:i}))}};
}
export function fixtures(){
 const list=[];
 for(const paint of ['alpha','transparent','gradient','image'])for(const shape of ['rect','curve','hole','stroke'])for(const huge of [false,true]){
  const shift=huge?1n<<80n:0n,world=(x,y)=>point(shift+BigInt(x),-shift+BigInt(y));
  const brush=paint==='gradient'?{kind:'gradient',gradient:{geometry:{kind:'linear',start:world(0,0),end:world(32,24)},stops:[{position:0,srgb:[1,0.2,0,0.5]},{position:1,srgb:[0,0.5,1,1]}],tile:'clamp',interpolation:'srgb',alpha:'premultiplied'}}:
   paint==='image'?{kind:'image',image:{resource:0,origin:world(0,0),xStep:point(4,0),yStep:point(0,3),tileX:'mirror',tileY:'repeat',sampling:'nearest'}}:
   {kind:'solid',rgba:paint==='alpha'?[255,0,0,128]:[0,0,0,0]};
  const paths=[rect(0,0,32,24),rect(0,0,23,18),rect(2,1,30,23),rect(6,4,26,21)];
  if(shape==='curve')paths[1]=polygon([[0,0],[23,18],[0,18]]),paths[1].commands[1]={kind:'cubic',control1:point(22,0),control2:point(0,18),to:point(23,18)};
  if(shape==='hole'){paths[1].fillRule='evenodd';paths[1].commands.push(...rect(5,4,16,13).commands);}
  const clips=[{parent:null,path:2,origin:world(0,0)},{parent:null,path:3,origin:world(0,0)}];
  const draws=[
   {path:0,origin:world(0,0),brush,clip:paint==='gradient'||paint==='image'?0:null},
   {path:0,origin:world(0,0),brush:{kind:'solid',rgba:[0,170,0,255]}},
   {path:1,origin:world(3,2),brush:{kind:'snapshot',afterDraws:1},blend:'source',clip:1,
    ...(shape==='stroke'?{stroke:{width:fixed(3),cap:'round',join:{kind:'miterClip',limit:fixed(1)}}}:{})},
  ];
  const request={viewport:{width:32,height:24,origin:world(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:'16777216',background:[0,0,0,0]},paths,clips,draws};
  list.push({name:`${paint}-${shape}-${huge?'huge':'zero'}`,request,images:paint==='image',constant:paint==='alpha'||paint==='transparent'?brush:null,
   exactRectangle:shape==='rect',window:[6,4,26,20]});
 }
 const q=structuredClone(list[0].request);
 q.draws.push({path:0,origin:point(0,0),brush:{kind:'solid',rgba:[255,136,0,255]}},
  {path:1,origin:point(0,0),brush:{kind:'snapshot',afterDraws:1},blend:'source'},
  {path:1,origin:point(16,12),brush:{kind:'snapshot',afterDraws:3},blend:'source'},
  {path:1,origin:point(0,12),brush:{kind:'snapshot',afterDraws:0},blend:'source'});
 list.push({name:'three-prefixes-immutable',request:q,images:false});
 const over=structuredClone(list[0].request);over.draws[2].blend='sourceOver';
 list.push({name:'snapshot-source-over',request:over,images:false});
 return list;
}
