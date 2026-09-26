import { compile } from 'json-schema-to-typescript';
import { mkdir, readFile, readdir, writeFile, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { modules } from './split-types.mjs';

const mode = process.argv[2];
assert.ok(mode === 'write' || mode === 'check', 'usage: generate-types.mjs <write|check>');
const input = new URL('../../contracts/generated/', import.meta.url);
const output = new URL('../../packages/contracts/src/generated/', import.meta.url);
if (mode === 'write') await mkdir(output, { recursive: true });
for (const name of (await readdir(input)).filter(name => name.endsWith('.schema.json')).sort()) {
  const schema = JSON.parse(await readFile(new URL(name, input), 'utf8'));
  const code = await compile(schema, schema.title, {
    cwd: fileURLToPath(input),
    bannerComment: '/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */',
    additionalProperties: false,
    unreachableDefinitions: true,
    strictIndexSignatures: true,
    $refOptions: { resolve: { http: false } },
  });
  const stem=name.replace('.schema.json','');
  const files=modules(stem,code);
  for (const [file,text] of files) {
    const target=new URL(file,output);
    if (mode === 'write') {await mkdir(new URL('./',target),{recursive:true});await writeFile(target,text);}
    else assert.equal(await readFile(target,'utf8'),text,`generated TypeScript differs: ${file}`);
  }
  const directory=new URL(stem+'/',output);
  const existing=await readdir(directory).catch(e=>{if(e.code==='ENOENT')return [];throw e;});
  for (const file of existing) {
    assert.match(file,/^part-[0-9]+\.ts$/,`unexpected generated-directory file: ${stem}/${file}`);
    if (files.has(stem+'/'+file)) continue;
    const target=new URL(file,directory);
    assert.ok((await readFile(target,'utf8')).startsWith('/* Generated from Rust → JSON Schema → TypeScript.'),`refusing to remove non-generated ${file}`);
    if(mode==='write')await rm(target);
    else assert.fail(`stale generated module: ${stem}/${file}`);
  }
  console.log(`${mode} ${name.replace('.schema.json', '.ts')}`);
}
