import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {modules} from './split-types.mjs';

const tiny='export interface Tiny { value: string; }\n';
assert.deepEqual([...modules('tiny',tiny)],[['tiny.ts',tiny]]);
let code='/** Public first declaration documentation. */\nexport interface First { next?: Last; literal: "Last is only literal text"; }\n';
for(let i=0;i<380;i++)code+=`/** Source property ${i}. */\nexport interface Record${i} {\n a: number;\n b?: string;\n c: First;\n d: Record${(i+1)%380} | null;\n}\n`;
code+='/** Last declaration documentation. */\nexport interface Last extends First { owner?: Record379; }\n';
const files=modules('probe',code);assert.ok(files.size>=4);
assert.ok([...files.values()].some(s=>s.includes('Public first declaration documentation.')));
assert.ok([...files.values()].every(s=>s.split('\n').length<=2000));
assert.deepEqual([...modules('probe',code)],[...files]);
const root=fs.mkdtempSync('.codex-work/source-text/split-test-');
try {
 for(const [name,text]of files){const file=path.join(root,name);fs.mkdirSync(path.dirname(file),{recursive:true});fs.writeFileSync(file,text);}
 fs.writeFileSync(path.join(root,'test.ts'),`import type {First,Last,Record0,Record379} from './probe.js';
declare const r:Record0;const n:number=r.a;const next:Last|undefined=r.c.next;
declare const z:Record379;const back:Record0|null=z.d;
// @ts-expect-error split declarations retain numeric field types
const invalid:string=r.a;
const first:First={literal:'Last is only literal text'};
void [n,next,back,invalid,first];\n`);
 fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'NodeNext',moduleResolution:'NodeNext'},include:['**/*.ts']}));
 const run=spawnSync('node',['node_modules/typescript/bin/tsc','-p',path.join(root,'tsconfig.json')],{encoding:'utf8',timeout:30000});assert.equal(run.status,0,run.stdout+run.stderr);
}finally{fs.rmSync(root,{recursive:true});}
assert.throws(()=>modules('bad','export class Unsupported {}\n'+'\n'.repeat(2001)),/only generated type/);
assert.throws(()=>modules('large','export interface Large {\n'+Array.from({length:2100},(_,i)=>`p${i}:number;\n`).join('')+'}\n'),/single declaration/);
console.log(JSON.stringify({compiler:'7.0.2',declarations:382,modules:files.size,cyclicImportsAndHeritageChecked:true,publicEntryChecked:true,negativeTypeAssignmentChecked:true,deterministic:true,oversizeAndUnexpectedDeclarationsRejected:true}));
