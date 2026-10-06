import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const [nativePath,wasmPath,clientPath,reportPath]=process.argv.slice(2);
assert(reportPath,'usage: video-operations-parity.mjs native wasm.js client.js report.json');
const module=createRequire(import.meta.url)(resolve(wasmPath));
const {WasmVideo,VideoComputationError}=await import(pathToFileURL(resolve(clientPath)).href);
const sdk=new WasmVideo(module);
const time=n=>({ticks:String(n),timescale:1});
const create={documentId:'authoring',title:'示例工程',rootSequence:'main',canvas:{width:1280,height:720},frameRate:{numerator:25,denominator:1},audioSampleRate:48000};
const initial=[{type:'addTrack',sequence:'main',track:{id:'v',name:'画面',kind:'video',locked:false,muted:false,solo:false,clips:[]},before:null},
  ...[['a',0,2],['b',3,4],['c',6,7]].map(([name,start,end])=>({type:'insert',track:'v',rippleTracks:[],clip:{id:name,name,timeline:{start:time(start),end:time(end)},source:{kind:'solid',rgba:[255,0,0,255]},effects:[],audio:{rate:'preservePitch',reverse:'reverse',hold:'silence'}}}))];
// Get the Rust-defined default audio policy from the schema-backed empty-source
// fixture contract: no JS timing or editing reducer is implemented here.
for(const op of initial) if(op.clip) delete op.clip.audio;
const edit=[{type:'setTitle',title:'新标题'}];
const query={kind:'clips',track:'v',range:{start:time(2),end:time(8)}};
const raw=JSON.stringify({create,initial,edit,query});
const native=spawnSync(resolve(nativePath),['--operations'],{input:raw,encoding:'utf8',maxBuffer:8<<20});
assert.equal(native.status,0,native.stderr);const expected=JSON.parse(native.stdout);assert(!expected.error,JSON.stringify(expected));
const handles=[];const own=h=>(handles.push(h),h);
let result;
try {
  const empty=own(sdk.create(create));
  const initialCandidate=empty.prepare({contract:'musteroffice.video.edit/1',expectedDigest:empty.summary().documentDigest,operations:initial});
  assert.equal(empty.summary().clipCount,0,'prepare cannot mutate a saved handle');
  const project=own(sdk.open(initialCandidate.snapshot));
  let cursor=null;const pages=[];
  do {const page=project.list({query,cursor,limit:1});pages.push(page);cursor=page.next;} while(cursor);
  const candidate=project.prepare({contract:'musteroffice.video.edit/1',expectedDigest:project.summary().documentDigest,operations:edit});
  const changed=own(sdk.open(candidate.snapshot));
  const inverse=changed.prepareInverse({expectedDigest:changed.summary().documentDigest,inverse:candidate.inverse});
  assert.deepEqual(inverse.snapshot,project.snapshot());
  let invalid;
  try {project.frame({expectedDigest:project.summary().documentDigest,frame:'-1'});assert.fail('outside frame accepted');}
  catch(error) {assert(error instanceof VideoComputationError);invalid=error.failure;}
  result={initial:initialCandidate,empty:empty.summary(),summary:project.summary(),pages,edit:candidate,inverse,resources:project.resources(),frame:project.frame({expectedDigest:project.summary().documentDigest,frame:'1'}),invalid};
  assert.deepEqual(result,expected);
  assert.deepEqual(pages.map(p=>p.items[0].clip.id),['b','c']);
  assert.throws(()=>changed.list({query,cursor:pages[0].next,limit:1}),e=>e instanceof VideoComputationError&&e.failure.code==='STALE_SNAPSHOT');
  const corrupt=project.snapshot();corrupt.document.title='forged';
  assert.throws(()=>sdk.open(corrupt),e=>e instanceof VideoComputationError&&e.failure.code==='SNAPSHOT_CORRUPT');
  assert.throws(()=>module.VideoProject.open('{"format":"musteroffice.video.snapshot/99"}'),e=>JSON.parse(e).code==='INPUT_INVALID');
  for(let i=0;i<20;i++) {const reopened=sdk.open(project.snapshot());assert.deepEqual(reopened.summary(),project.summary());reopened.close();reopened.close();assert.throws(()=>reopened.summary(),/closed/);}
} finally {for(const handle of handles) handle.close();}
const sha=v=>createHash('sha256').update(v).digest('hex');
const report={format:'musteroffice.video-operations-parity/1',status:'passed',fixtureSha256:sha(raw),resultSha256:sha(JSON.stringify(result)),
  wasmSha256:sha(readFileSync(resolve(wasmPath.replace(/\.js$/,'_bg.wasm')))),wasmBytes:readFileSync(resolve(wasmPath.replace(/\.js$/,'_bg.wasm'))).length,
  nativeSha256:sha(readFileSync(resolve(nativePath))),scope:'Real independent WASM authoring component and thin TS client match native snapshots, candidates, inverse, exact frame plans, pages and structured errors. No durable host commit, media device, full editing features or product acceptance.'};
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
