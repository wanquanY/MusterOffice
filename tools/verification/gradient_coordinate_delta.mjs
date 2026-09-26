/** Compare observed pixels, without treating old coordinates as a new correctness oracle. */
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
export function compareRenderedMetadata(current,previous,pixels,oldPixels){
 const a=JSON.parse(current),b=JSON.parse(previous);
 const hash=v=>createHash('sha256').update(v).digest('hex');
 function visit(now,old){
  if(!now||typeof now!=='object')return;
  if('frameSha256' in now&&'byteLength' in now&&'sha256' in now){
   assert.equal(now.sha256,hash(pixels));assert.equal(old.sha256,hash(oldPixels));
   old.sha256=now.sha256;
  }
  for(const key of Object.keys(now))visit(now[key],old?.[key]);
 }
 visit(a,b);assert.deepEqual(a,b);
}
export function pixelDelta(actual,previous){
 assert.equal(actual.length,previous.length);assert.equal(actual.length%4,0);
 let differentPixels=0,changedChannels=0,maxChannelDifference=0;const examples=[];
 for(let i=0;i<actual.length;i+=4){let differs=false;
  for(let k=0;k<4;k++){const d=Math.abs(actual[i+k]-previous[i+k]);changedChannels+=d!==0;differs ||= d!==0;maxChannelDifference=Math.max(maxChannelDifference,d);}
  if(differs){differentPixels++;if(examples.length<8)examples.push({pixel:i/4,current:[...actual.subarray(i,i+4)],previous:[...previous.subarray(i,i+4)]});}
 }
 return {differentPixels,changedChannels,maxChannelDifference,examples};
}
