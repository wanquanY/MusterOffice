/** Image parameter uncertainty through the real shared path and scene APIs. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import factory from '../../.codex-work/clips/component/mo-skia.mjs';
const root='.codex-work/image-paint',out=root+'/precision';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(path,data)=>{fs.writeFileSync(path,data);return entry(path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/image-paint/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/clips/component/mo-skia.wasm')));
const images=Buffer.from([255,0,0,255,0,255,0,255,0,0,255,255,255,255,0,255]);
const resources=[{width:2,height:2,alpha:'premultiplied',sha256:createHash('sha256').update(images).digest('hex')}];
const U=1n<<32n,f=n=>String(BigInt(n)*U),p=(x,y)=>({x:f(x),y:f(y)}),ratio=(n,d)=>String(BigInt(n)*U/BigInt(d));
const rectangle={fillRule:'nonzero',commands:[[0,0],[32,0],[32,24],[0,24]].map(([x,y],i)=>({kind:i?'line':'move',to:p(x,y)})).concat([{kind:'close'}])};
const zero=()=>({origin:p(0,0),xStep:p(0,0),yStep:p(0,0),sourceDomain:['0','0','0','0']});
const matrices=[[[1,3],[0,1],[0,1],[1,2]],[[1,3],[1,7],[-1,9],[1,2]],
 [[-1,3],[1,7],[1,9],[1,2]],[[0,1],[-1,3],[1,2],[0,1]],
 [[8,1],[2,1],[1,1],[4,1]],[[1,128],[1,1024],[-1,1024],[1,64]]];
function request(matrix,tiles,mode,domain,huge){
 const shift=huge?1n<<80n:0n,world=p(shift,-shift),m=matrix.map(([n,d])=>ratio(n,d));
 const image={resource:0,origin:world,xStep:{x:m[0],y:m[2]},yStep:{x:m[1],y:m[3]},tileX:tiles[0],tileY:tiles[1],sampling:'linear'};
 if(domain)image.sourceDomain={left:ratio(1,3),top:ratio(-1,5),right:ratio(7,4),bottom:ratio(5,2)};
 if(mode){image.uncertainty=zero();if(mode===2){image.uncertainty.origin={x:'100',y:'200'};image.uncertainty.xStep={x:'3',y:'2'};image.uncertainty.yStep={x:'4',y:'1'};if(domain)image.uncertainty.sourceDomain=['3','4','5','6'];}}
 return {viewport:{width:huge?8192:64,height:24,origin:world,scale:{numerator:1,denominator:1},coordinateTolerance:'16777216',background:[0,0,0,0]},paths:[rectangle],clips:[],draws:[{path:0,origin:world,brush:{kind:'image',image}}]};
}
function scene(q){return {viewport:q.viewport,scene:{paths:q.paths,transforms:[{parent:null,affine:{linear:[f(1),'0','0',f(1)],translation:q.draws[0].origin}}],clips:[],instances:q.draws.map(({origin,...d})=>({...d,transform:0}))}};}
let paired=0,calls=0,frame;
const backend={rasterImages(f,b){calls++;frame=f.slice();return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
function run(q,isScene){
 const json=JSON.stringify({raster:isScene?scene(q):q,images:resources}),data=Buffer.from(json),h=Buffer.alloc(8);h.writeUInt32LE(data.length);h.writeUInt32LE(images.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',[isScene?'--image-scene':'--images'],{input:Buffer.concat([h,data,images]),env:{},timeout:60000,maxBuffer:90*1024*1024});assert.equal(n.status,0,n.stderr.toString());
 const length=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+length).toString(),pixels=n.stdout.subarray(8+length);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 calls=0;frame=null;const w=isScene?wasm.render_image_scene(json,images,backend):wasm.render_image_paths(json,images,backend);
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!component.invalid);paired++;
 return {json,metadata,response:JSON.parse(metadata),pixels,frame,calls};
}
const cases=[];
for(let i=0;i<matrices.length;i++)for(const tiles of [['clamp','clamp'],['repeat','mirror'],['mirror','repeat'],['decal','repeat']])for(const domain of [false,true])for(const huge of [false,true]){
 let reference;
 for(const mode of [0,1,2]){
  const q=request(matrices[i],tiles,mode,domain,huge),name=`m${i}-${tiles.join('-')}-d${+domain}-h${+huge}-u${mode}`,r=run(q,false),s=run(q,true);
  assert.equal(r.response.status,'rendered',name+': '+r.metadata);assert.equal(s.response.status,'rendered');assert.equal(r.calls,1);assert.equal(s.calls,1);assert.deepEqual(s.pixels,r.pixels);
  if(reference){assert.deepEqual(r.frame,reference.frame,name);assert.deepEqual(r.pixels,reference.pixels,name);if(mode===1)assert.equal(r.metadata,reference.metadata,name);}
  else reference=r;
  const b=out+'/'+name;cases.push({name,mode,request:put(b+'.json',r.json),sceneRequest:put(b+'.scene.json',s.json),response:put(b+'.response.json',r.metadata),sceneResponse:put(b+'.scene-response.json',s.metadata),pixels:put(b+'.rgba',r.pixels),frame:put(b+'.frame',Buffer.from(r.frame.buffer))});
 }
}
const failures=[];
const base=request(matrices[0],['repeat','repeat'],2,true,true);
for(const [name,mutate] of [
 ['tight-affine',q=>{q.viewport.coordinateTolerance='16384';delete q.draws[0].brush.image.uncertainty;delete q.draws[0].brush.image.sourceDomain;}],
 ...['origin','xStep','yStep'].flatMap(key=>['x','y'].map(axis=>[`negative-${key}-${axis}`,q=>q.draws[0].brush.image.uncertainty[key][axis]='-1'])),
 ...[0,1,2,3].map(i=>[`negative-domain-${i}`,q=>q.draws[0].brush.image.uncertainty.sourceDomain[i]='-1']),
 ['implicit-domain',q=>delete q.draws[0].brush.image.sourceDomain],
 ['collapsed-period',q=>q.draws[0].brush.image.uncertainty.sourceDomain[2]=f(10)],
 ['singular-input',q=>q.draws[0].brush.image.uncertainty.xStep.x=f(1)],
 ['out-of-range',q=>q.draws[0].brush.image.uncertainty.xStep.x=String(1n<<126n)],
 ['unknown-field',q=>q.draws[0].brush.image.uncertainty.ignored=true],
 ['missing-axis',q=>delete q.draws[0].brush.image.uncertainty.xStep.y],
]){
 const q=structuredClone(base);mutate(q);
 for(const isScene of [false,true]){const r=run(q,isScene);assert.equal(r.response.status,'error',name);assert.equal(r.calls,0,name);assert.equal(r.pixels.length,0);const b=out+'/'+name+(isScene?'-scene':'');failures.push({name,isScene,request:put(b+'.invalid.json',r.json),response:put(b+'.invalid-response.json',r.metadata)});}
}
const result={format:'musteroffice.image-precision-parity/1',pairedCalls:paired,cases,failures,images:put(out+'/images.rgba',images),artifacts:['target/debug/mo-raster-worker',root+'/wasm-node/mo_wasm_bg.wasm',root+'/wasm-node/mo_wasm.js','.codex-work/clips/component/mo-skia.wasm','.codex-work/clips/component/mo-skia.mjs','.codex-work/clips/ts-raster/index.js'].map(entry)};
fs.writeFileSync(root+'/precision.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({cases:cases.length,pairedCalls:paired,preflightFailures:failures.length}));
