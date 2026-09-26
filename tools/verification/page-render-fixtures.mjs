// Owned author pages exercising the real document-to-raster entry.
import fs from 'node:fs';
import {strokePageFixtures} from './stroke-page-fixtures.mjs';
const read=p=>JSON.parse(fs.readFileSync(p));
export const rgba=(red,green,blue,alpha=255)=>({red,green,blue,alpha});
export const solid=color=>({kind:'value',value:{kind:'solid',color:{kind:'srgb',rgba:color}}});
export function base(){
 const d=read('fixtures/presentations/basic-shape.json');d.pageSize={width:'800',height:'600'};
 const o=d.objects['shape:1'];o.content.text=null;o.transform.origin={x:'100',y:'120'};o.transform.size={width:'300',height:'160'};
 o.appearance={fill:solid(rgba(180,40,70)),stroke:{kind:'value',value:{kind:'none'}}};
 return {page:{document:d,slide:'slide:1'},viewport:{width:800,height:600,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:1},coordinateTolerance:'16777216',background:[0,0,0,0]},defaults:{themeColors:{},pageBackground:rgba(255,255,255)}};
}
export const shape=q=>q.page.document.objects['shape:1'];
function layers(q){
 const d=q.page.document;d.slides['slide:1'].layout='layout:1';d.slides['slide:1'].background={kind:'inherit'};
 d.themes={'theme:1':{id:'theme:1',name:'Owned',colors:{accent1:rgba(20,180,90)},defaultText:{}}};
 d.masters={'master:1':{id:'master:1',theme:'theme:1',objects:['master-shape'],background:solid(rgba(12,23,34)),defaultText:{}}};
 d.layouts={'layout:1':{id:'layout:1',name:'Owned',master:'master:1',objects:['layout-shape'],background:{kind:'inherit'},defaultText:{}}};
 for(const [id,kind,parent,x,color] of [['master-shape','master','master:1',0,rgba(255,0,0)],['layout-shape','layout','layout:1',50,rgba(0,0,255,128)]]){
  const o=structuredClone(shape(q));o.id=id;o.parent={kind,id:parent};o.transform.origin={x:String(x),y:'100'};o.appearance.fill=solid(color);d.objects[id]=o;
 }
 shape(q).appearance.fill={kind:'value',value:{kind:'solid',color:{kind:'theme',slot:'accent1'}}};return q;
}
export function fixtures(){
 const out=[];function add(name,mutate=()=>{},expected='rendered',validRequest=true){const q=base();mutate(q);out.push({name,q,expected,validRequest});return q;}
 add('rectangle');
 for(const kind of ['rectangle','ellipse','roundRectangle'])for(const rotation of [0,1,2700000,5400000,-8100000,24300000,2147483647,-2147483648]){
  add(`${kind}-${rotation}`,q=>{const o=shape(q);o.content.geometry={kind,...(kind==='roundRectangle'?{radius:'61'}:{})};o.transform.rotation=rotation;});
 }
 for(const flip of [1,2,3])add('ellipse-flip-'+flip,q=>{shape(q).content.geometry={kind:'ellipse'};shape(q).transform.rotation=1234567;shape(q).transform.flipHorizontal=!!(flip&1);shape(q).transform.flipVertical=!!(flip&2);});
 for(const radius of ['0','80'])add('round-radius-'+radius,q=>shape(q).content.geometry={kind:'roundRectangle',radius});
 for(const [w,h]of [['0','160'],['300','0'],['0','0'],['301','159']])add(`ellipse-size-${w}-${h}`,q=>{shape(q).content.geometry={kind:'ellipse'};shape(q).transform.size={width:w,height:h};});
 add('custom-curves',q=>{shape(q).transform.rotation=1834567;shape(q).content.geometry={kind:'path',viewport:{width:'3',height:'7'},commands:[{kind:'move',to:{x:'0',y:'0'}},{kind:'quadratic',control:{x:'4',y:'-3'},to:{x:'3',y:'4'}},{kind:'cubic',control1:{x:'5',y:'7'},control2:{x:'-2',y:'8'},to:{x:'0',y:'0'}},{kind:'close'}]};});
 add('nonzero-holes',q=>shape(q).content.geometry={kind:'path',viewport:{width:'100',height:'100'},commands:[...[['move',0,0],['line',100,0],['line',100,100],['line',0,100]].map(([kind,x,y])=>({kind,to:{x:String(x),y:String(y)}})),{kind:'close'},...[['move',25,25],['line',25,75],['line',75,75],['line',75,25]].map(([kind,x,y])=>({kind,to:{x:String(x),y:String(y)}})),{kind:'close'}]});
 add('theme-layering',layers);
 add('layout-background',q=>{layers(q);q.page.document.layouts['layout:1'].background=solid(rgba(91,82,73));});
 add('slide-background',q=>{layers(q);q.page.document.slides['slide:1'].background=solid(rgba(91,82,73));});
 add('host-theme-fallback',q=>{shape(q).appearance.fill={kind:'value',value:{kind:'solid',color:{kind:'theme',slot:'accent2'}}};q.defaults.themeColors.accent2=rgba(37,98,151);});
 add('host-background',q=>q.page.document.slides['slide:1'].background={kind:'inherit'});
 add('alpha-composition',q=>{q.viewport.background=[255,255,255,255];shape(q).appearance.fill=solid(rgba(255,0,0,128));});
 add('none-fill',q=>shape(q).appearance.fill={kind:'value',value:{kind:'none'}});
 add('transparent-fill',q=>shape(q).appearance.fill=solid(rgba(123,241,53,0)));
 add('hidden-slide',q=>q.page.document.slides['slide:1'].hidden=true);
 add('off-page-clipped',q=>shape(q).transform.origin={x:'-100',y:'-80'});
 add('repeated-8192',q=>{const d=q.page.document,o=structuredClone(shape(q));d.objects={};d.slides['slide:1'].objects=[];for(let i=0;i<8192;i++){const c=structuredClone(o);c.id='repeated:'+i;d.objects[c.id]=c;d.slides['slide:1'].objects.push(c.id);}});
 add('empty-page',q=>{q.page.document.objects={};q.page.document.slides['slide:1'].objects=[];});
 add('rational-scale',q=>{q.viewport.width=1200;q.viewport.height=900;q.viewport.scale={numerator:3,denominator:2};shape(q).content.geometry={kind:'ellipse'};});
 add('tight-curve-budget',q=>{q.viewport.coordinateTolerance='65536';shape(q).content.geometry={kind:'ellipse'};},'PRECISION_EXCEEDED');
 add('usable-curve-budget',q=>{q.viewport.coordinateTolerance='262144';shape(q).content.geometry={kind:'ellipse'};});
 for(const name of ['angles','flips','nested']){
  const q=base();q.page=read(`.codex-work/angle-export/${name}.request.json`);q.viewport.scale={numerator:1,denominator:9525};out.push({name:'group-'+name,q,expected:'rendered',validRequest:true,priorPixels:`.codex-work/angle-export/${name}.rgba`});
 }
 for(const [name,mutate,code,valid]of [
  ['shape-text',q=>shape(q).content.text=read('fixtures/presentations/basic-shape.json').objects['shape:1'].content.text,'MAPPING_NOT_IMPLEMENTED',true],
  ['stroke',q=>shape(q).appearance.stroke={kind:'value',value:{kind:'solid',width:'10',color:{kind:'srgb',rgba:rgba(0,0,0)}}},'MAPPING_NOT_IMPLEMENTED',true],
  ['inherited-fill',q=>delete shape(q).appearance.fill,'MAPPING_NOT_IMPLEMENTED',true],
  ['inherited-stroke',q=>delete shape(q).appearance.stroke,'MAPPING_NOT_IMPLEMENTED',true],
  ['missing-theme',q=>shape(q).appearance.fill={kind:'value',value:{kind:'solid',color:{kind:'theme',slot:'accent1'}}},'INPUT_INVALID',true],
  ['radius-too-large',q=>shape(q).content.geometry={kind:'roundRectangle',radius:'81'},'INPUT_INVALID',true],
  ['negative-radius',q=>shape(q).content.geometry={kind:'roundRectangle',radius:'-1'},'INPUT_INVALID',true],
  ['device-range',q=>shape(q).transform.origin.x='1000000000','COORDINATE_RANGE',true],
  ['precision',q=>{shape(q).content.geometry={kind:'ellipse'};q.viewport.coordinateTolerance='256';},'PRECISION_EXCEEDED',true],
  ['viewport-origin',q=>q.viewport.origin.x='1','INPUT_INVALID',true],
  ['viewport-crop',q=>q.viewport.width=799,'INPUT_INVALID',true],
  ['viewport-nonuniform',q=>q.viewport.scale.numerator=2,'INPUT_INVALID',true],
  ['zero-scale',q=>q.viewport.scale.denominator=0,'INPUT_INVALID',true],
  ['negative-size',q=>shape(q).transform.size.width='-1','INPUT_INVALID',true],
  ['unknown-slide',q=>q.page.slide='missing','INPUT_INVALID',true],
  ['unknown-field',q=>q.unknown=true,'INPUT_INVALID',false],
  ['numeric-emu',q=>shape(q).transform.origin.x=2,'INPUT_INVALID',false],
 ])add(name,mutate,code,valid);
 for(const name of ['picture','connector']){
  const original=read('fixtures/presentations/native-export/request.json').document;
  const q=base(),id=name==='picture'?'picture:1':'connector:2';
  shape(q).content=structuredClone(original.objects[id].content);
  if(name==='picture')q.page.document.resources=original.resources;
  else {shape(q).content.start={kind:'free',position:{x:'0',y:'0'}};shape(q).content.end={kind:'free',position:{x:'100',y:'100'}};}
  out.push({name:'unsupported-'+name,q,expected:'MAPPING_NOT_IMPLEMENTED',validRequest:true});
 }
 out.push({name:'duplicate-key',q:'{"page":{},"page":{}}',expected:'INPUT_INVALID',validRequest:false});
 return [...out,...strokePageFixtures(base,shape)];
}
