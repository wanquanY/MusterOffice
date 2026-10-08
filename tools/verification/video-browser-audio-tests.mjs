import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFile, mkdtemp, writeFile, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { build } from 'esbuild';
import { DEFAULT_MEDIA_LIMITS } from '../../.codex-work/video-browser-media/build/video-browser-media/src/contract.js';
import { BrowserMediaBudget } from '../../.codex-work/video-browser-media/build/video-browser-media/src/media-budget.js';
import { BrowserVideoBudget } from '../../.codex-work/video-browser-media/build/video-browser-media/src/video-budget.js';

test('audio and existing video aliases share the same admission ledger',()=>{
  assert.equal(BrowserVideoBudget,BrowserMediaBudget);
  const pool=new BrowserMediaBudget(2048),releaseAudio=pool.reserve(1536),releaseVideo=pool.reserve(512);
  assert.throws(()=>pool.reserve(1),e=>e.code==='resource_limit');releaseAudio();releaseVideo();assert.equal(pool.usedBytes,0);
});
test('AAC configuration admits actual ASC rate/layout/frame size and rejects incompatible profiles before codec creation',async()=>{
  const root=fileURLToPath(new URL('../../',import.meta.url));
  const spec=JSON.parse(await readFile(path.join(root,'components/video-browser-media/component.json')));
  const vendor=path.join(root,'.codex-work/components/mediabunny',spec.sourceCommit),hash=createHash('sha256');
  for(const name of [...spec.sourceFiles].sort())hash.update(name+'\0').update(await readFile(path.join(vendor,name))).update('\0');
  assert.equal(hash.digest('hex'),spec.sourceTreeSha256);
  const directory=await mkdtemp(path.join(root,'.codex-work/video-audio-unit-'));
  try{
    const built=await build({absWorkingDir:root,entryPoints:['packages/video-browser-media/src/audio-platform.ts'],bundle:true,format:'esm',platform:'node',write:false,
      plugins:[{name:'pinned-aac-parser',setup(build){build.onResolve({filter:/^@musteroffice\/mediabunny-aac$/},()=>({path:path.join(vendor,'shared/aac-misc.ts')}));}}]});
    const filename=path.join(directory,'aac.mjs');await writeFile(filename,built.outputFiles[0].contents);
    const {aacConfiguration}=await import(pathToFileURL(filename).href);
    const config={codec:'mp4a.40.2',sampleRate:48000,numberOfChannels:1,description:Uint8Array.from([0x11,0x88,0x56,0xe5,0])};
    const actual=aacConfiguration(config,DEFAULT_MEDIA_LIMITS);assert.equal(actual.frameSamples,1024);assert.equal(actual.frameBytes,4096);
    assert.equal(actual.sampleRate,48000);assert.equal(actual.channels,1);assert.notEqual(actual.description,config.description);
    const short={...config,description:Uint8Array.from([0x11,0x8c])};assert.equal(aacConfiguration(short,DEFAULT_MEDIA_LIMITS).frameSamples,960);
    const stereo={...config,sampleRate:44100,numberOfChannels:2,description:Uint8Array.from([0x12,0x10])};
    assert.equal(aacConfiguration(stereo,DEFAULT_MEDIA_LIMITS).frameBytes,8192);
    const rejected=(change,code='unsupported_codec',limits=DEFAULT_MEDIA_LIMITS)=>assert.throws(()=>aacConfiguration({...config,...change},limits),e=>e.code===code);
    rejected({description:undefined});rejected({codec:'mp4a.40.5'});rejected({codec:'opus'});
    rejected({description:Uint8Array.from([0x11,0x80])}); // PCE-only channel map
    rejected({description:Uint8Array.from([0x11,0x89])}); // extension flags
    rejected({description:Uint8Array.from([0x11,0x8a])}); // dependent core coder
    rejected({sampleRate:44100},'unsupported_media');rejected({numberOfChannels:2},'unsupported_media');
    rejected({description:new Uint8Array(4097)},'resource_limit');rejected({},'resource_limit',{...DEFAULT_MEDIA_LIMITS,maxAudioBlockBytes:4095});
    rejected({},'resource_limit',{...DEFAULT_MEDIA_LIMITS,maxAudioSampleRate:44100});
    // Hierarchical SBR configuration must not enter the LC-only clock/capacity.
    rejected({description:Uint8Array.from([0x2b,0x92,0x08,0])});
  }finally{await rm(directory,{recursive:true,force:true});}
});
