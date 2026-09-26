import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/page-placement';fs.mkdirSync(root,{recursive:true});
const sha=x=>createHash('sha256').update(x).digest('hex'),read=p=>JSON.parse(fs.readFileSync(p));
const cases=[];
function base(){const document=read('fixtures/presentations/basic-shape.json');document.objects['shape:1'].content.text=null;document.objects['shape:1'].transform.size={width:'200',height:'100'};return{document,slide:'slide:1'};}
function group(q,depth=1){const d=q.document;let children=['shape:1'];for(let i=0;i<depth;i++){
 const id='group:'+i,o=structuredClone(d.objects['shape:1']);o.id=id;o.parent={kind:'slide',id:q.slide};o.transform={origin:{x:'10',y:'20'},size:{width:'400',height:'150'},rotation:0,flipHorizontal:false,flipVertical:false};o.content={kind:'group',children,viewport:{width:'200',height:'100'}};
 for(const id of children)d.objects[id].parent={kind:'group',id:o.id};d.objects[id]=o;children=[id];
 }d.slides[q.slide].objects=children;return q;}
function run(name,q,expected='evaluated',validRequest=true){
 const json=typeof q==='string'?q:JSON.stringify(q),requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`;fs.writeFileSync(requestPath,json);
 const n=spawnSync('target/release/mo-cli',['page-placements',requestPath],{encoding:'utf8',maxBuffer:32*1024*1024,timeout:30000});assert.equal(n.status,0,n.stderr);
 const native=n.stdout.trimEnd(),w=wasm.page_placements(json);assert.equal(native,w,name);const response=JSON.parse(w);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 fs.writeFileSync(responsePath,w);cases.push({name,requestPath,responsePath,requestSha256:sha(json),responseSha256:sha(w),status:expected,validRequest});return response;
}
run('base',base());
for(const angle of [0,1,-1,2699999,2700000,2700001,5399999,5400000,5400001,10800000,16200000,21599999,21600000,-21600001,2147483647,-2147483648]){
 const q=base();q.document.objects['shape:1'].transform.rotation=angle;run('angle-'+angle,q);
}
for(const angle of [0,1800000,5400000,-3600000])for(const flip of ['none','h','v','both']){
 const q=group(base());const g=q.document.objects['group:0'];g.transform.rotation=angle;g.transform.flipHorizontal=['h','both'].includes(flip);g.transform.flipVertical=['v','both'].includes(flip);q.document.objects['shape:1'].transform.rotation=2700000;run(`nested-${angle}-${flip}`,q);
}
const random=base(),template=structuredClone(random.document.objects['shape:1']);random.document.objects={};random.document.slides['slide:1'].objects=[];
let seed=19;function next(){seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed;}
for(let i=0;i<400;i++){const o=structuredClone(template);o.id='random:'+i;o.transform.rotation=(next()%21600000)-10800000;o.transform.flipHorizontal=!!(next()&1);o.transform.flipVertical=!!(next()&2);o.transform.origin={x:String(next()%100000-50000),y:String(next()%100000-50000)};o.transform.size={width:String(next()%100000),height:String(next()%100000)};random.document.objects[o.id]=o;random.document.slides['slide:1'].objects.push(o.id);}
run('angle-grid',random);
const custom=group(base(),3);custom.document.objects['shape:1'].content.geometry={kind:'path',viewport:{width:'3',height:'7'},commands:[{kind:'move',to:{x:'-1',y:'2'}},{kind:'cubic',control1:{x:'4',y:'-3'},control2:{x:'9',y:'20'},to:{x:'2',y:'3'}},{kind:'close'}]};
for(const [i,o]of Object.values(custom.document.objects).entries()){o.transform.rotation=1234567*i-321321;o.transform.flipVertical=!!(i&1);o.transform.size={width:String(13+2*i),height:String(17+i)};}
run('custom-rational-nesting',custom);
for(const [name,x,w]of [['large-integer','9007199254740993','239'],['minimum-origin','-9223372036854775808','9223372036854775807'],['maximum-origin','9223372036854775806','1']]){const q=base();const t=q.document.objects['shape:1'].transform;t.origin.x=x;t.size.width=w;t.rotation=2345678;run(name,q);}
for(const [w,h]of [['0','17'],['19','0'],['0','0'],['21','19']]){const q=base();q.document.objects['shape:1'].transform.size={width:w,height:h};run(`size-${w}-${h}`,q);}
for(const depth of [128,129]){const q=group(base(),depth);for(const o of Object.values(q.document.objects)){o.transform.size={width:'200',height:'100'};o.transform.rotation=65432;}run('depth-'+depth,q,depth===128?'evaluated':'LIMIT_EXCEEDED');}
for(const count of [8192,8193]){const q=base();q.document.objects={};q.document.slides['slide:1'].objects=[];for(let i=0;i<count;i++){const o=structuredClone(template);o.id='count:'+i;q.document.objects[o.id]=o;q.document.slides['slide:1'].objects.push(o.id);}run('count-'+count,q,count===8192?'evaluated':'LIMIT_EXCEEDED');}
const nativeDocument=read('fixtures/presentations/native-export/request.json').document;
for(const slide of nativeDocument.slideOrder)run('native-export-'+slide.replace(':','-'),{document:nativeDocument,slide});
const hidden=base();hidden.document.slides['slide:1'].hidden=true;assert.equal(run('hidden-slide',hidden).result.hidden,true);
for(const [angle,flip]of [[0,'none'],[1800000,'h'],[5400000,'v'],[-3600000,'both']]){
 const q=group(base());q.document.pageSize={width:'7620000',height:'5715000'};
 const g=q.document.objects['group:0'];g.transform.rotation=angle;g.transform.flipHorizontal=['h','both'].includes(flip);g.transform.flipVertical=['v','both'].includes(flip);
 g.transform.origin={x:'1428750',y:'1905000'};
 g.transform.size={width:'3810000',height:'1428750'};g.content.viewport={width:'1905000',height:'952500'};
 const o=q.document.objects['shape:1'];o.transform.origin={x:'190500',y:'95250'};o.transform.size={width:'1428750',height:'666750'};o.transform.rotation=2700000;
 o.appearance={fill:{kind:'value',value:{kind:'solid',color:{kind:'srgb',rgba:{red:180,green:40,blue:70,alpha:255}}}},stroke:{kind:'value',value:{kind:'none'}}};
 q.document.slides['slide:1'].background={kind:'value',value:{kind:'solid',color:{kind:'srgb',rgba:{red:255,green:255,blue:255,alpha:255}}}};
 run(`diagnostic-${angle}-${flip}`,q);
}
const empty=base();empty.document.objects={};empty.document.slides['slide:1'].objects=[];run('empty-slide',empty);
for(const [name,change,expected,valid]of [
 ['unknown-slide',q=>q.slide='slide:missing','INPUT_INVALID',true],
 ['ownership',q=>q.document.objects['shape:1'].parent={kind:'group',id:'missing'},'INPUT_INVALID',true],
 ['negative-size',q=>q.document.objects['shape:1'].transform.size.width='-1','INPUT_INVALID',true],
 ['bounds-overflow',q=>q.document.objects['shape:1'].transform.origin.x='9223372036854775807','INPUT_INVALID',true],
 ['angle-overflow',q=>q.document.objects['shape:1'].transform.rotation=2147483648,'INPUT_INVALID',false],
 ['numeric-emu',q=>q.document.objects['shape:1'].transform.size.width=23,'INPUT_INVALID',false],
 ['unknown-field',q=>q.document.extra=true,'INPUT_INVALID',false],
]){const q=base();change(q);run(name,q,expected,valid);}
const range=group(base(),3);for(const o of Object.values(range.document.objects)){o.transform.origin={x:'0',y:'0'};o.transform.size={width:'4611686018427387904',height:'10'};if(o.content.kind==='group')o.content.viewport.width='1';}run('coordinate-range',range,'COORDINATE_RANGE');
const zero=group(base());zero.document.objects['group:0'].content.viewport.width='0';run('zero-group-viewport',zero,'INPUT_INVALID');
run('duplicate-json-key','{"slide":"slide:1","slide":"slide:2","document":{}}','INPUT_INVALID',false);
const report={format:'musteroffice.page-placement-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,evaluated:cases.filter(c=>c.status==='evaluated').length}));
