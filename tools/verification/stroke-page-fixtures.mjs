// All cases exercise the author document compiler, not handcrafted Draw IR.
export function strokePageFixtures(base,shape){
 const cases=[];
 function add(name,edit=()=>{},expected='rendered',validRequest=true){
  const q=base(),o=shape(q);
  o.appearance.fill={kind:'value',value:{kind:'none'}};
  o.appearance.stroke={kind:'value',value:{kind:'solid',width:'12',cap:'flat',join:{kind:'round'},color:{kind:'srgb',rgba:{red:180,green:40,blue:70,alpha:255}}}};
  o.content.geometry={kind:'path',viewport:{width:'100',height:'100'},commands:[{kind:'move',to:{x:'10',y:'80'}},{kind:'line',to:{x:'50',y:'10'}},{kind:'line',to:{x:'90',y:'80'}}]};
  edit(q,o,o.appearance.stroke.value);cases.push({name:'stroke-author-'+name,q,expected,validRequest});return q;
 }
 for(const cap of ['flat','round','square']){
  for(const join of ['round','bevel','miter'])add(cap+'-'+join,(_q,_o,s)=>{s.cap=cap;s.join={kind:join,...(join==='miter'?{limit:400000}:{})};});
  for(const width of ['0','1','50'])add(cap+'-width-'+width,(_q,_o,s)=>{s.cap=cap;s.width=width;});
  add(cap+'-zero-segment',(_q,o,s)=>{s.cap=cap;o.content.geometry.commands=[{kind:'move',to:{x:'50',y:'50'}},{kind:'line',to:{x:'50',y:'50'}}];});
 }
 for(const limit of [0,100000,141421,141422,200000,314159,800000,102400000])add('miter-'+limit,(_q,_o,s)=>s.join={kind:'miter',limit});
 for(const kind of ['rectangle','ellipse','roundRectangle'])for(const fill of [false,true])add(kind+(fill?'-fill':''),(_q,o)=>{
  o.content.geometry={kind,...(kind==='roundRectangle'?{radius:'45'}:{})};
  if(fill)o.appearance.fill={kind:'value',value:{kind:'solid',color:{kind:'srgb',rgba:{red:0,green:80,blue:230,alpha:255}}}};
 });
 add('alpha',(_q,o,s)=>{s.color.rgba.alpha=128;o.appearance.fill={kind:'value',value:{kind:'solid',color:{kind:'srgb',rgba:{red:0,green:0,blue:255,alpha:255}}}};});
 add('closed',(_q,o)=>o.content.geometry.commands.push({kind:'close'}));
 add('quadratic',(_q,o)=>o.content.geometry.commands=[{kind:'move',to:{x:'0',y:'50'}},{kind:'quadratic',control:{x:'50',y:'0'},to:{x:'100',y:'50'}}]);
 add('cubic',(_q,o)=>o.content.geometry.commands=[{kind:'move',to:{x:'0',y:'50'}},{kind:'cubic',control1:{x:'40',y:'-50'},control2:{x:'60',y:'150'},to:{x:'100',y:'50'}}]);
 add('theme',(q,_o,s)=>{s.color={kind:'theme',slot:'accent2'};q.defaults.themeColors.accent2={red:80,green:150,blue:40,alpha:255};});
 add('rotated',(_q,o)=>o.transform.rotation=1234567);
 for(const [sx,sy,flip] of [[1,1,false],[2,1,false],[1,2,false],[2,2,false],[2,1,true]])add(`group-${sx}-${sy}-${flip}`,(q,o)=>{
  const d=q.page.document,g=structuredClone(o);g.id='stroke-group';g.content={kind:'group',children:[o.id],viewport:{width:'400',height:'300'}};
  g.transform.origin={x:'0',y:'0'};g.transform.size={width:String(400*sx),height:String(300*sy)};g.transform.flipHorizontal=flip;g.appearance={};
  o.parent={kind:'group',id:g.id};d.objects[g.id]=g;d.slides['slide:1'].objects=[g.id];
 });
 add('viewport-scale',q=>{q.viewport.width=1200;q.viewport.height=900;q.viewport.scale={numerator:3,denominator:2};});
 add('none-ignores-no-parameters',(_q,o)=>o.appearance.stroke={kind:'value',value:{kind:'none'}});
 for(const [name,edit,code,valid] of [
  ['unresolved-cap',(_q,_o,s)=>delete s.cap,'MAPPING_NOT_IMPLEMENTED',true],
  ['unresolved-join',(_q,_o,s)=>delete s.join,'MAPPING_NOT_IMPLEMENTED',true],
  ['unresolved-miter-limit',(_q,_o,s)=>s.join={kind:'miter'},'MAPPING_NOT_IMPLEMENTED',true],
  ['negative-width',(_q,_o,s)=>s.width='-1','INPUT_INVALID',true],
  ['width-range',(_q,_o,s)=>s.width='32769','COORDINATE_RANGE',true],
  ['miter-range',(_q,_o,s)=>s.join={kind:'miter',limit:102400001},'INPUT_INVALID',true],
  ['missing-theme',(_q,_o,s)=>s.color={kind:'theme',slot:'accent4'},'INPUT_INVALID',true],
  ['round-parameter',(_q,_o,s)=>s.join={kind:'round',limit:1},'INPUT_INVALID',false],
  ['negative-miter',(_q,_o,s)=>s.join={kind:'miter',limit:-1},'INPUT_INVALID',false],
  ['none-parameter',(_q,o)=>o.appearance.stroke={kind:'value',value:{kind:'none',cap:'flat'}},'INPUT_INVALID',false],
 ])add(name,edit,code,valid);
 return cases;
}
