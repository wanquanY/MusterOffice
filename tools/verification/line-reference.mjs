import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
export function verifyLineReference(q,result,sources) {
 let referenceFragments=0,referenceGlyphs=0;
 const paragraph=q.paragraph,chars=[...paragraph.text];
 for(const line of result.lines)for(let i=line.fallbackStart;i<line.fallbackEnd;i++) {
  const item=result.items[result.shapedItemIndices[i]],style=paragraph.styles[item.style];
  for(const fragment of result.fallback.items[i].fragments) {
   if(fragment.status!=='selected')continue;
   const {start,end,shaped}=fragment,candidate=style.candidates[fragment.candidate],font=paragraph.fonts[candidate.font];
   const args=[sources.get(font.expectedSha256),'--text='+chars.slice(start,end).join(''),
    '--text-before='+chars.slice(line.start.scalarOffset,start).join(''),'--text-after='+chars.slice(end,line.end.scalarOffset).join(''),
    '--direction='+(item.level%2?'rtl':'ltr'),'--script='+item.script,'--language='+style.language,'--face-index='+font.faceIndex,
    '--font-size='+shaped.positionUnitsPerEm,'--font-funcs=ot','--shapers=ot','--cluster-level=0','--unsafe-to-concat','--no-glyph-names','--show-flags','-O','json'];
   if(start===line.start.scalarOffset)args.push('--bot');if(end===line.end.scalarOffset)args.push('--eot');
   assert.equal(style.suppressDottedCircle,false);
   if(candidate.variations.length)args.push('--variations='+candidate.variations.map(v=>`${v.tag}=${Math.fround(v.value1616/65536)}`).join(','));
   const features=style.features.flatMap(f=>{const a=Math.max(start,f.start),b=Math.min(end,f.end??chars.length);return a<b?[`${f.tag}[${a-start}:${b-start}]=${f.value}`]:[];});
   if(features.length)args.push('--features='+features.join(','));
   const ref=spawnSync('.codex-work/harfbuzz/release/hb-shape-reference',args,{encoding:'utf8',timeout:30000});assert.equal(ref.status,0,ref.stderr);
   const expected=JSON.parse(ref.stdout).map(g=>({...g,fl:g.fl??0,cl:g.cl+start}));
   const actual=shaped.runs[0].glyphs.map(g=>({g:g.glyphId,cl:g.cluster,fl:(g.unsafeToBreak?1:0)|(g.unsafeToConcat?2:0)|(g.safeToInsertTatweel?4:0),ax:g.xAdvance,ay:g.yAdvance,dx:g.xOffset,dy:g.yOffset}));
   assert.deepEqual(actual,expected,'line fragment differs from upstream');referenceFragments++;referenceGlyphs+=actual.length;
  }
 }
 return {referenceFragments,referenceGlyphs};
}
