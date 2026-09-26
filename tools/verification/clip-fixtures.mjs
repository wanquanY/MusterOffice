/** Owned evaluated geometry; no office assets or implicit fonts. */
const U=1n<<32n;
export const fixed=n=>(BigInt(n)*U).toString();
export const point=(x,y)=>({x:fixed(x),y:fixed(y)});
const path=(points,fillRule='nonzero')=>({fillRule,commands:[...points.map(([x,y],i)=>({kind:i?'line':'move',to:point(x,y)})),{kind:'close'}]});
const rect=(l,t,r,b)=>path([[l,t],[r,t],[r,b],[l,b]]);
const inside=(box,x,y)=>x>=box[0]&&y>=box[1]&&x<box[2]&&y<box[3];
export function fixtures(){
 const list=[];
 for(const paint of ['solid','gradient','image','stroke'])for(const mode of ['rect','nested','branches','hole','empty','deep'])for(const enormous of [false,true]){
  const shift=enormous?1n<<80n:0n, world=(x,y)=>point(shift+BigInt(x),-shift+BigInt(y));
  const brush=paint==='gradient'?{kind:'gradient',gradient:{geometry:{kind:'linear',start:world(0,0),end:world(32,24)},stops:[{position:0,srgb:[1,0.2,0,1]},{position:1,srgb:[0,0.5,1,1]}],tile:'clamp',interpolation:'srgb',alpha:'premultiplied'}}:
    paint==='image'?{kind:'image',image:{resource:0,origin:world(2,1),xStep:point(2,0),yStep:point(0,2),tileX:'mirror',tileY:'repeat',sampling:'linear',sourceDomain:{left:fixed(1),top:fixed(0),right:fixed(4),bottom:fixed(3)}}}:{kind:'solid',rgba:[210,90,20,255]};
  const paths=[rect(0,0,32,24),rect(2,3,26,21),rect(6,1,30,17),rect(12,7,29,23)];
  let clips=[{parent:null,path:1,origin:world(0,0)}],selection=[0],predicates=[(x,y)=>inside([2,3,26,21],x,y)];
  if(mode==='nested'||mode==='branches'){
   clips.push({parent:0,path:2,origin:world(0,0)},{parent:0,path:3,origin:world(0,0)});
   predicates.push((x,y)=>inside([6,1,30,17],x,y),(x,y)=>inside([12,7,29,23],x,y));
   selection=mode==='nested'?[1]:[1,1,2,null,1,0,2];
  }
  if(mode==='hole'){
   paths[1].fillRule='evenodd';paths[1].commands.push(...rect(8,8,17,15).commands);
   predicates[0]=(x,y)=>inside([2,3,26,21],x,y)&&!inside([8,8,17,15],x,y);
  }
  if(mode==='empty'){paths[1].commands=[];predicates[0]=()=>false;}
  if(mode==='deep'){
   clips=Array.from({length:64},(_,i)=>({parent:i?i-1:null,path:1,origin:world(0,0)}));
   predicates=clips.map(()=>predicates[0]);selection=[63,63,32,63,null,63];
  }
  const draws=selection.map((clip,i)=>({path:0,origin:world(0,0),clip,brush:paint==='solid'?{kind:'solid',rgba:[30+i*20,180-i*15,40+i*20,255]}:structuredClone(brush),
   ...(paint==='stroke'?{stroke:{width:fixed(8),cap:'round',join:{kind:'miterClip',limit:fixed(1)}}}:{})}));
  const viewport={width:32,height:24,origin:world(0,0),scale:{numerator:1,denominator:1},coordinateTolerance:'16777216',background:[0,0,0,0]};
  const request={viewport,paths,clips,draws};
  // Change the source coordinates and apply a genuine quarter-turn + huge
  // translation only to clips. This yields the same world intersection.
  const clipPaths=paths.slice(1).map(p=>({...p,commands:p.commands.map(c=>c.to?{...c,to:{x:c.to.y,y:(-BigInt(c.to.x)).toString()}}:c)}));
  const scene={viewport,scene:{paths:[paths[0],...clipPaths],transforms:[{parent:null,affine:{linear:[fixed(1),'0','0',fixed(1)],translation:world(0,0)}},{parent:null,affine:{linear:['0',fixed(-1),fixed(1),'0'],translation:world(0,0)}}],
   clips:clips.map(c=>({parent:c.parent,path:c.path,transform:1})),instances:draws.map(({origin,...d})=>({...d,transform:0}))}};
  const mask=(draw,x,y)=>{let i=draw.clip;while(i!==null&&i!==undefined){if(!predicates[i](x,y))return false;i=clips[i].parent;}return true;};
  list.push({name:`${paint}-${mode}-${enormous?'huge':'zero'}`,request,scene,mask,images:paint==='image'});
 }
 // Curved AA clipping is paired across runtimes, without claiming an analytic
 // pixel oracle for Skia's curve coverage algorithm.
 for(const evenodd of [false,true]){
  const c=structuredClone(list[0].request);c.paths[1]={fillRule:evenodd?'evenodd':'nonzero',commands:[{kind:'move',to:point(2,2)},{kind:'cubic',control1:point(30,1),control2:point(2,23),to:point(30,22)},{kind:'line',to:point(2,22)},{kind:'close'},...rect(8,8,17,15).commands]};
  const scene={viewport:c.viewport,scene:{paths:c.paths,transforms:[],clips:c.clips.map(({origin,...v})=>({...v,transform:null})),instances:c.draws.map(({origin,...v})=>({...v,transform:null}))}};
  list.push({name:'curve-'+evenodd,request:c,scene,images:false});
 }
 return list;
}
