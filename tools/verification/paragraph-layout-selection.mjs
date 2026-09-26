// Exhaustive forward candidate enumeration through the separate explicit-line
// geometry API. Independent selection logic and BigInt arithmetic; shares the
// validated shaper/line analysis. Not an independent text layout engine.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHarness} from './paragraph-harness.mjs';
const {wasm,component,sha}=await createHarness('.codex-work/paragraph-layout');
const root='.codex-work/paragraph-layout',report=JSON.parse(readFileSync(root+'/parity.json'));
const UNIT=1n<<32n;
function nearest(n,d){const sign=n<0n?-1n:1n,a=n<0n?-n:n;return sign*((2n*a+d)/(2n*d));}
const scale=(n,size,denom)=>nearest(BigInt(n)*BigInt(size)*UNIT,BigInt(denom));
const proofs=[];let candidateCalls=0;
for(const c of report.cases) {
 const q=JSON.parse(readFileSync(c.requestPath)),response=JSON.parse(readFileSync(c.responsePath));
 if(response.status!=='evaluated'||!response.result.geometry||[...q.paragraph.text].length>64)continue;
 const r=response.result,n=[...q.paragraph.text].length,bundle=readFileSync(c.bundlePath);
 const segment=JSON.parse(wasm.analyze_text(JSON.stringify({texts:[q.paragraph.text],characters:[]}))).texts[0];
 const boundaries=segment.boundaries.map(b=>b.scalarOffset),set=new Set(boundaries);
 const opportunities=r.breaks.opportunities.filter(b=>set.has(b.boundary.scalarOffset));
 const measured=new Map();
 function measure(start,end) {
  const key=start+':'+end;if(measured.has(key))return measured.get(key);
  if(start===end){const v={start,end,min:'0',max:'0',fits:true};measured.set(key,v);return v;}
  const lineEnds=[...new Set([start,end,n].filter(v=>v>0))].sort((a,b)=>a-b);
  const gq={shaping:{paragraph:q.paragraph,lineEnds},styles:q.styles,strutStyle:q.strutStyle,spacing:q.spacing};
  const answer=JSON.parse(wasm.layout_lines(JSON.stringify(gq),bundle,component));assert.equal(answer.status,'evaluated',c.name);
  const shaped=answer.result.shaping,k=shaped.lines.findIndex(l=>l.start.scalarOffset===start&&l.end.scalarOffset===end);assert.ok(k>=0);
  const line=shaped.lines[k],ranks=new Map(shaped.bidi.lines[k].visualOrder.map((s,i)=>[s,i])),parts=[];
  for(let i=line.fallbackStart;i<line.fallbackEnd;i++) {
   const item=shaped.items[shaped.shapedItemIndices[i]],style=q.styles[item.style];
   for(const f of shaped.fallback.items[i].fragments) {
    assert.equal(f.status,'selected');let rank=Infinity;
    for(let s=f.start;s<f.end;s++)if(ranks.has(s))rank=Math.min(rank,ranks.get(s));
    if(rank!==Infinity)parts.push({rank,style,shaped:f.shaped});
   }
  }
  parts.sort((a,b)=>a.rank-b.rank);
  let x=0n,min=0n,max=0n;
  for(const p of parts){let prefix=0n;for(const g of p.shaped.runs[0].glyphs){prefix+=BigInt(g.xAdvance);const pen=x+scale(prefix,p.style.fontSize,p.shaped.positionUnitsPerEm);min=pen<min?pen:min;max=pen>max?pen:max;}x+=scale(prefix,p.style.fontSize,p.shaped.positionUnitsPerEm);}
  const v={start,end,min:String(min),max:String(max),fits:min>=0n&&max<=BigInt(q.width)*UNIT};measured.set(key,v);candidateCalls++;return v;
 }
 const expected=[];let start=0;
 if(n===0)expected.push({end:0,emergency:false,overflows:false});
 while(start<n) {
  const mandatory=opportunities.find(b=>b.kind==='mandatory'&&b.boundary.scalarOffset>start).boundary.scalarOffset;
  const legal=opportunities.filter(b=>b.boundary.scalarOffset>start&&b.boundary.scalarOffset<=mandatory).map(b=>b.boundary.scalarOffset);
  const all=legal.map(end=>measure(start,end));const fit=all.filter(v=>v.fits);
  let chosen=fit.at(-1),emergency=false;
  if(!chosen&&q.overflow==='emergencyGrapheme') {
   const emergencyOptions=boundaries.filter(end=>end>start&&end<legal[0]).map(end=>measure(start,end));
   chosen=emergencyOptions.filter(v=>v.fits).at(-1)??emergencyOptions[0];emergency=!!chosen;
  }
  chosen??=all[0];assert.ok(chosen.end>start);expected.push({end:chosen.end,emergency,overflows:!chosen.fits});start=chosen.end;
 }
 if(/[\u000b\u000c\u2028]$/u.test(q.paragraph.text))expected.push({end:n,emergency:false,overflows:false});
 assert.deepEqual(r.decisions.map(d=>({end:d.end.scalarOffset,emergency:d.emergency,overflows:d.overflows})),expected,c.name);
 proofs.push({name:c.name,decisions:expected,candidates:[...measured.values()]});
}
const output={format:'musteroffice.paragraph-selection-reference/1',rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),layouts:proofs.length,candidateCalls,
 scope:'Exhaustive ascending enumeration and BigInt Q32 pen arithmetic using separate explicit-line geometry entry; shared shaping/bidi. Long resource-limit corpora excluded from enumeration.',proofs};
writeFileSync(root+'/selection.json',JSON.stringify(output,null,2)+'\n');console.log(JSON.stringify({layouts:proofs.length,candidateCalls}));
