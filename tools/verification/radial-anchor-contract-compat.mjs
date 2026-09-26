/** Removing just the new profile restores the former Rust-generated contract. */
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {compile} from 'json-schema-to-typescript';
import {modules} from '../contracts/split-types.mjs';
const root='.codex-work/radial-observation';
const previous='docs/reviews/evidence/2026-09-25-radial-layout-verification.json';
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const prior=new Map(JSON.parse(fs.readFileSync(previous)).sourceFiles.map(r=>[r.path,r]));
const name='pptx-radial-layout-request',schemaPath='contracts/generated/'+name+'.schema.json';
const schema=JSON.parse(fs.readFileSync(schemaPath));
assert.deepEqual(schema.$defs.RadialLayoutProfile.enum,['drawingml-circle-path-bounds-q96-v1-draft','drawingml-circle-anchor-focus-q96-v2-draft']);
schema.$defs.RadialLayoutProfile.enum.pop();
const digest=createHash('sha256').update(JSON.stringify(schema,null,2)+'\n').digest('hex');assert.equal(digest,prior.get(schemaPath).sha256);
const code=await compile(schema,schema.title,{cwd:path.resolve('contracts/generated'),bannerComment:'/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */',additionalProperties:false,unreachableDefinitions:true,strictIndexSignatures:true,$refOptions:{resolve:{http:false}}});
const restored=[];
for(const [file,text] of modules(name,code)){
 const p='packages/contracts/src/generated/'+file;assert.equal(createHash('sha256').update(text).digest('hex'),prior.get(p).sha256);
 restored.push({historicalPath:p,byteLength:prior.get(p).byteLength,sha256:prior.get(p).sha256});
}
assert.equal(restored.length,1);
fs.writeFileSync(root+'/contract-compat.json',JSON.stringify({format:'musteroffice.radial-anchor-contract-compat/1',previousEvidence:entry(previous),schema:entry(schemaPath),priorSchemaSha256:digest,restoredTypes:restored},null,2)+'\n');
console.log(JSON.stringify({extendedSchemas:1,restoredTypes:restored.length}));
