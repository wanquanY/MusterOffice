/** Rebuild prior TypeScript from precisely reverted schemas, including file splitting. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {compile} from 'json-schema-to-typescript';
import {modules} from '../contracts/split-types.mjs';
const root='.codex-work/elliptic-source',previous='docs/reviews/evidence/2026-09-25-elliptic-render-verification.json';
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const old=JSON.parse(fs.readFileSync(previous)),prior=new Map(old.sourceFiles.map(r=>[r.path,r])),records=[];
for(const name of fs.readdirSync('contracts/generated').filter(n=>n.endsWith('.schema.json')).sort()){
 const filename='contracts/generated/'+name,schema=JSON.parse(fs.readFileSync(filename));
 const field=schema.$defs?.GradientField,work=schema.$defs?.RasterWork;
 if(!field&&!work){assert.deepEqual(entry(filename),prior.get(filename));continue;}
 if(field){assert.equal(field.oneOf.length,3);assert.equal(field.oneOf[0].properties.kind.const,'elliptic');field.oneOf.shift();}
 if(work){
  assert(work.properties.ellipticGradients);assert(!work.required.includes('ellipticGradients'));
  delete work.properties.ellipticGradients;delete schema.$defs.EllipticGradientWork;
  work.properties.gradientValueErrorBound.description='Maximum absolute f64 -> f32 error of stop positions and color channels.';
 }
 const bytes=JSON.stringify(schema,null,2)+'\n',digest=createHash('sha256').update(bytes).digest('hex');assert.equal(digest,prior.get(filename).sha256);
 const code=await compile(schema,schema.title,{cwd:path.resolve('contracts/generated'),bannerComment:'/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */',additionalProperties:false,unreachableDefinitions:true,strictIndexSignatures:true,$refOptions:{resolve:{http:false}}});
 const files=modules(name.replace('.schema.json',''),code),restored=[];
 for(const [filename,text] of files){const key='packages/contracts/src/generated/'+filename;assert.equal(createHash('sha256').update(text).digest('hex'),prior.get(key).sha256);restored.push({historicalPath:key,byteLength:prior.get(key).byteLength,sha256:prior.get(key).sha256});}
 records.push({schema:entry(filename),priorSchemaSha256:prior.get(filename).sha256,restoredTypeFiles:restored});
}
assert.equal(records.length,14);
fs.writeFileSync(root+'/contract-compat.json',JSON.stringify({format:'musteroffice.elliptic-source-contract-compat/1',previousEvidence:entry(previous),scope:'Removing the new elliptic field, optional work metadata and its explanatory documentation restores each previous schema and generated TypeScript file byte for byte. New TypeScript module splitting follows the unchanged generator.',cases:records},null,2)+'\n');
console.log(JSON.stringify({extendedSchemas:records.length,reconstructedPriorTypeFiles:records.reduce((a,c)=>a+c.restoredTypeFiles.length,0)}));
