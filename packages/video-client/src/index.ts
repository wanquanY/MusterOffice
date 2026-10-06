import type { CreateProject } from '../../contracts/src/generated/video-project-create.js';
import type { VideoSnapshot } from '../../contracts/src/generated/video-snapshot.js';
import type { ProjectSummary } from '../../contracts/src/generated/video-project-summary.js';
import type { MutationCandidate } from '../../contracts/src/generated/video-mutation-candidate.js';
import type { PageRequest } from '../../contracts/src/generated/video-page-request.js';
import type { ProjectPage } from '../../contracts/src/generated/video-project-page.js';
import type { FrameRequest } from '../../contracts/src/generated/video-frame-request.js';
import type { VideoFramePlan } from '../../contracts/src/generated/video-frame-plan.js';
import type { InverseRequest } from '../../contracts/src/generated/video-inverse-request.js';
import type { Transaction } from '../../contracts/src/generated/video-transaction.js';
import type { ResourceClosure } from '../../contracts/src/generated/video-resource-closure.js';
import type { Failure } from '../../contracts/src/generated/video-operation-failure.js';
export type { CreateProject,VideoSnapshot,ProjectSummary,MutationCandidate,PageRequest,ProjectPage,FrameRequest,VideoFramePlan,InverseRequest,Transaction,ResourceClosure,Failure };
export const VIDEO_PROFILE = 'musteroffice.video.operations/1-draft';
export interface VideoHandle {
  snapshot():string; summary():string; resources():string;
  list(input:string):string; frame(input:string):string;
  prepare(input:string):string; prepareInverse(input:string):string;
  free():void;
}
export interface VideoModule {
  video_profile():string;
  VideoProject:{create(input:string):VideoHandle;open(input:string):VideoHandle};
}
export class VideoComputationError extends Error {
  constructor(readonly failure:Failure) {super(failure.message);this.name='VideoComputationError';}
}
function call<T>(run:()=>T):T {
  try {return run();} catch(error) {
    if(typeof error==='string') {
      let value:unknown;
      try {value=JSON.parse(error);} catch {throw new Error('Invalid video component failure envelope');}
      if(value && typeof value==='object' && 'code' in value && typeof value.code==='string' && 'message' in value && typeof value.message==='string') {
        throw new VideoComputationError(value as Failure);
      }
      throw new Error('Invalid video component failure envelope');
    }
    throw error;
  }
}
/** Thin synchronous facade intended for a host-owned computation Worker.
 * No loading, network, DOM, persistence, task scheduler or JS editing reducer. */
export class WasmVideo {
  constructor(private readonly module:VideoModule) {
    if(module.video_profile()!==VIDEO_PROFILE) throw new Error('Video component profile mismatch');
  }
  create(input:CreateProject):VideoProject {return new VideoProject(call(()=>this.module.VideoProject.create(JSON.stringify(input))));}
  open(snapshot:VideoSnapshot):VideoProject {return new VideoProject(call(()=>this.module.VideoProject.open(JSON.stringify(snapshot))));}
}
export class VideoProject {
  private handle:VideoHandle|undefined;
  /** Create through WasmVideo; the handle has one owner and is explicitly freed. */
  constructor(handle:VideoHandle) {this.handle=handle;}
  private read<T>(run:(handle:VideoHandle)=>string):T {
    if(!this.handle) throw new Error('Video project handle is closed');
    return JSON.parse(call(()=>run(this.handle!))) as T;
  }
  snapshot():VideoSnapshot {return this.read(h=>h.snapshot());}
  summary():ProjectSummary {return this.read(h=>h.summary());}
  resources():ResourceClosure {return this.read(h=>h.resources());}
  list(request:PageRequest):ProjectPage {return this.read(h=>h.list(JSON.stringify(request)));}
  frame(request:FrameRequest):VideoFramePlan {return this.read(h=>h.frame(JSON.stringify(request)));}
  prepare(transaction:Transaction):MutationCandidate {return this.read(h=>h.prepare(JSON.stringify(transaction)));}
  prepareInverse(request:InverseRequest):MutationCandidate {return this.read(h=>h.prepareInverse(JSON.stringify(request)));}
  close():void {const handle=this.handle;this.handle=undefined;handle?.free();}
}
