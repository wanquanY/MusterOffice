/** Wrap retained gradient/image/clip material in one isolated opacity group. */
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const [out,manifest,newPath,oldPath,tsPath]=process.argv.slice(2);
assert(out&&manifest&&newPath&&oldPath&&tsPath);await fs.mkdir(out,{recursive:false});
const {RasterComponent}=await import(pathToFileURL(path.resolve(tsPath,'index.js')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const component=async p=>{
 const {default:factory}=await import(pathToFileURL(path.resolve(p,'mo-skia.mjs')));
 const bytes=await fs.readFile(path.join(p,'mo-skia.wasm'));
 return [await RasterComponent.create(factory,await WebAssembly.compile(bytes)),sha(bytes)];
};
const [current,currentSha]=await component(newPath),[old,oldSha]=await component(oldPath),seen=new Set(),cases=[];
async function load(record){const b=await fs.readFile(record.path);assert.equal(b.length,record.byteLength);assert.equal(sha(b),record.sha256);return b;}
async function save(name,bytes){const p=path.join(out,name);await fs.writeFile(p,bytes,{flag:'wx'});return {path:p,byteLength:bytes.length,sha256:sha(bytes)};}
const call=(c,f,b)=>b?c.rasterImages(f,b):c.raster(f);
for(const c of JSON.parse(await fs.readFile(manifest)).cases){
 if(c.status||!c.frame)continue;
 const data=await load(c.frame),frame=new Uint32Array(Uint8Array.from(data).buffer),version=frame[1],key=version+'-'+Boolean(c.images);
 if(version<7||version>12||!frame[6]||seen.has(key))continue;seen.add(key);
 const images=c.images?Uint8Array.from(await load(c.images)):undefined;
 const original=call(current,frame,images);assert.equal(original.status,0);assert.deepEqual(Buffer.from(original.pixels),await load(c.currentPixels));
 const transparent=frame.slice();transparent[4]=0;
 const baseline=call(old,transparent,images);assert.equal(baseline.status,0);
 const words=Array.from(frame),header=version>=8?14:13,drawWords=version>=8?8:7,drawAt=words.length-words[6]*drawWords;
 const groups=[0,frame[6],32768];
 const prefix=[...words.slice(0,header),...(version<8?[0]:[]),1,...words.slice(header,drawAt),...groups];
 for(let d=0;d<words[6];d++)prefix.push(...words.slice(drawAt+d*drawWords,drawAt+(d+1)*drawWords),...(version<8?[0]:[]));
 prefix[1]=13;const wrapped=new Uint32Array(prefix);
 assert.throws(()=>call(old,wrapped,images),/opacity group extension/);
 assert.throws(()=>old.beginRaster(wrapped,images),/opacity group extension/);
 const reply=call(current,wrapped,images);assert.equal(reply.status,0);
 const rgba=[frame[4]&255,frame[4]>>>8&255,frame[4]>>>16&255,frame[4]>>>24];
 const backdrop=rgba.map((v,i)=>i===3?v:Math.floor((v*rgba[3]+127)/255));
 for(let i=0;i<reply.pixels.length;i+=4){
  const s=baseline.pixels.subarray(i,i+4),a=Math.round(s[3]*32768/65535);
  for(let k=0;k<4;k++)assert.equal(reply.pixels[i+k],Math.round(s[k]*32768/65535)+Math.round(backdrop[k]*(255-a)/255),`${key}/${i}/${k}`);
 }
 const n=cases.length;
 cases.push({name:'advanced/'+key,status:0,source:c.frame,frame:await save(n+'.bin',Buffer.from(wrapped.buffer)),currentPixels:await save(n+'.rgba',reply.pixels),...(c.images?{images:c.images}:{}),oracle:'Unmodified prior component on transparent destination, followed by independently computed group coverage and source-over.'});
}
assert.equal(cases.length,8);
const report={format:'musteroffice.opacity-group-components/1',status:'passed',currentSha,oldSha,cases};
await save('report.json',Buffer.from(JSON.stringify(report,null,2)+'\n'));console.log(JSON.stringify({status:report.status,cases:cases.length}));
