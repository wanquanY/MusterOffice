/** Actual Native/WASM calls over owned delivered bytes and adversarial variants. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const [directory, cli, modulePath] = process.argv.slice(2);
assert(directory && cli && modulePath);
fs.mkdirSync(directory);
const root='fixtures/presentations/delivery-receive';
const original=JSON.parse(fs.readFileSync(root+'/request.json'));
const originalBytes=fs.readFileSync(root+'/assets.bin');
const sha=b=>createHash('sha256').update(b).digest('hex');
const artifact=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const wasm=createRequire(import.meta.url)(resolve(modulePath));
const cases=[];
const run=(name,edit,expected='INPUT_INVALID')=>{
  const request=structuredClone(original);
  const data=new Map(original.contents.map(c=>[c.assetId,Buffer.from(originalBytes.subarray(Number(c.byteOffset),Number(c.byteOffset)+Number(c.byteLength)))]));
  const asset=mime=>request.bundle.assets.find(a=>a.mediaType===mime);
  const replace=(id,bytes)=>{
    const a=request.bundle.assets.find(a=>a.id===id);assert(a);data.set(id,bytes);
    a.sha256=sha(bytes);a.byteLength=String(bytes.length);
  };
  const json=(id,fn)=>{const v=JSON.parse(data.get(id));fn(v);replace(id,Buffer.from(JSON.stringify(v)));};
  const state={request,data,asset,replace,json};
  const wireEdit=edit?.(state);
  let offset=0;
  request.contents=[...data].map(([id,b])=>{
    const binding={assetId:id,byteOffset:String(offset),byteLength:String(b.length)};offset+=b.length;return binding;
  });
  const bytes=Buffer.concat([...data.values()]);
  let input=JSON.stringify(request);
  if(wireEdit)input=wireEdit(request,input);
  const inputPath=`${directory}/${name}.json`, bytesPath=`${directory}/${name}.bin`;
  fs.writeFileSync(inputPath,input,{flag:'wx'});fs.writeFileSync(bytesPath,bytes,{flag:'wx'});
  const native=spawnSync(cli,['delivery-inspect',inputPath,bytesPath],{encoding:'utf8',timeout:30000,maxBuffer:4*1024*1024});
  assert.equal(native.status,0,native.stderr);assert.equal(native.stderr,'');
  const a=JSON.parse(native.stdout), b=JSON.parse(wasm.inspect_delivery(input,bytes));
  assert.deepEqual(a,b,name);
  if(expected==='inspected') {
    assert.equal(a.status,'inspected',JSON.stringify(a));assert.equal(a.report.assetsVerified,12);
    assert.equal(a.report.pages,2);assert.deepEqual(a.report.declaredClaims,request.bundle.claims);
  } else {assert.equal(a.status,'error',name);assert.equal(a.error.code,expected,`${name}: ${JSON.stringify(a)}`);}
  const responsePath=`${directory}/${name}.response.json`;
  fs.writeFileSync(responsePath,JSON.stringify(a,null,2)+'\n',{flag:'wx'});
  cases.push({name,expected,input:artifact(inputPath),bytes:artifact(bytesPath),response:artifact(responsePath)});
};
run('valid',null,'inspected');
run('reordered-bindings',()=>r=>{r.contents.reverse();return JSON.stringify(r);},'inspected');
run('reordered-physical-content',({data})=>{const a=[...data].reverse();data.clear();for(const [id,b] of a)data.set(id,b);},'inspected');
run('unknown-field',()=>r=>JSON.stringify({...r,authority:'not-permission'}));
run('duplicate-json-field',()=> (_r,s)=>'{"bundle":{},'+s.slice(1));
run('numeric-length',()=>r=>{r.contents[0].byteLength=Number(r.contents[0].byteLength);return JSON.stringify(r);});
run('noncanonical-length',()=>r=>{r.contents[0].byteLength='00';return JSON.stringify(r);});
run('overflowing-range',()=>r=>{r.contents[0].byteOffset='18446744073709551615';return JSON.stringify(r);});
run('overlapping-ranges',()=>r=>{r.contents[1].byteOffset='0';return JSON.stringify(r);});
run('range-gap',()=>r=>{r.contents[0].byteOffset='1';return JSON.stringify(r);});
run('missing-range',()=>r=>{r.contents.pop();return JSON.stringify(r);});
run('duplicate-range',()=>r=>{r.contents[1].assetId=r.contents[0].assetId;return JSON.stringify(r);});
run('unknown-range',()=>r=>{r.contents[0].assetId='asset:other';return JSON.stringify(r);});
run('trailing-unbound-content',({data})=>{data.set('asset:unused',Buffer.from('extra'));return r=>{r.contents.pop();return JSON.stringify(r);};});
run('future-version',({request:r})=>{r.bundle.version='musteroffice.bundle/99';});
run('foreign-document-pin',({request:r})=>{r.expected.documentId='document:other';});
run('foreign-revision-pin',({request:r})=>{r.expected.revision='0'.repeat(64);});
run('foreign-semantic-pin',({request:r})=>{r.expected.semanticDigest='0'.repeat(64);});
run('duplicate-asset',({request:r})=>{r.bundle.assets.push(structuredClone(r.bundle.assets[0]));});
run('asset-budget',({request:r})=>{r.bundle.assets[0].byteLength='134217729';},'LIMIT_EXCEEDED');
run('corrupt-font-bytes',({data,asset})=>{data.get(asset('application/octet-stream').id)[0]^=1;});
run('model-semantic-content',({asset,json})=>json(asset('application/vnd.musteroffice.presentation+json').id,v=>{v.document.title='foreign content';}));
const ctx='application/vnd.musteroffice.presentation-context+json';
run('context-renderer-pin',({asset,json})=>json(asset(ctx).id,v=>{v.previewRenderer.implementationSha256='0'.repeat(64);}));
run('context-settings-pin',({asset,json})=>json(asset(ctx).id,v=>{v.settings.previewWidth=320;}));
run('context-resource-closure',({asset,json})=>json(asset(ctx).id,v=>{v.resourceAssets={};}));
run('context-font-closure',({asset,json})=>json(asset(ctx).id,v=>{v.fontBundleAssetId=null;}));
run('context-duplicate-field',({asset,data,replace})=>{const id=asset(ctx).id;replace(id,Buffer.from('{"format":"duplicate",'+data.get(id).toString().slice(1)));});
run('quality-promotion',({asset,json})=>json(asset('application/vnd.musteroffice.quality+json').id,v=>{v.targetApplicationProven=true;}));
run('claim-promotion',({request:r})=>{r.bundle.claims[4].status='passed';r.bundle.claims[4].basis='application-test';});
run('swapped-page-order',({request:r})=>{r.bundle.previews.reverse();});
run('duplicate-preview-evidence',({asset,json})=>json(asset('application/vnd.musteroffice.quality+json').id,v=>{v.previews[1]=v.previews[0];}));
run('foreign-preview-source',({asset,json})=>json(asset('application/json').id,v=>{v.render.page.page.sourceSha256='0'.repeat(64);}));
run('foreign-preview-pixels',({asset,json})=>json(asset('application/json').id,v=>{v.render.page.scene.raster.sha256='0'.repeat(64);}));
run('swapped-png-bytes-with-matching-descriptor',({request:r,data,replace,asset,json})=>{
  const id=r.bundle.previews[0].imageAssetId;replace(id,data.get(r.bundle.previews[1].imageAssetId));
  const a=r.bundle.assets.find(a=>a.id===id);json(asset('application/json').id,v=>{v.previewAsset=a;});
});
run('png-bad-crc-with-matching-descriptor',({request:r,data,replace,asset,json})=>{
  const id=r.bundle.previews[0].imageAssetId,b=Buffer.from(data.get(id));b[40]^=1;replace(id,b);
  const a=r.bundle.assets.find(a=>a.id===id);json(asset('application/json').id,v=>{v.previewAsset=a;});
});
run('png-bad-deflate-with-valid-crc',({request:r,data,replace,asset,json})=>{
  const id=r.bundle.previews[0].imageAssetId,b=Buffer.from(data.get(id));
  let offset=8;
  while(b.subarray(offset+4,offset+8).toString()!=='IDAT')offset+=12+b.readUInt32BE(offset);
  const length=b.readUInt32BE(offset);assert(length>2);b[offset+8]=0;
  let crc=0xffffffff;
  for(const byte of b.subarray(offset+4,offset+8+length)) {
    crc^=byte;for(let bit=0;bit<8;bit++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);
  }
  b.writeUInt32BE((crc^0xffffffff)>>>0,offset+8+length);replace(id,b);
  const a=r.bundle.assets.find(a=>a.id===id);json(asset('application/json').id,v=>{v.previewAsset=a;});
});
const report={format:'musteroffice.delivery-receive-parity/1',native:artifact(cli),wasmGlue:artifact(modulePath),
  wasm:artifact(resolve(modulePath,'../mo_wasm_bg.wasm')),cases,
  limitations:'Owned two-page bundle and negative variants. This is byte/reference binding, not source-to-output visual equivalence or Office/WPS/Musterwork acceptance.'};
fs.writeFileSync(`${directory}/report.json`,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({cases:cases.length,accepted:cases.filter(c=>c.expected==='inspected').length,report:`${directory}/report.json`}));
