import type { HostRequest, OperationRequest } from '../../contracts/src/generated/host-request.js';
import type { HostResponse, HostResult, JobInfo } from '../../contracts/src/generated/host-response.js';
import type { CallOptions, OperationPort, WaitOptions } from './ports.js';
import { checkAbort, ClientError, delay, requireState, waiting } from './errors.js';
import { positiveBound } from './scalars.js';

export type JobResponse = Exclude<HostResponse, { outcome: 'succeeded' }> |
  { outcome: 'succeeded'; result: Extract<HostResult, { kind: 'job' }> };

function jobResponse(response: HostResponse, expected: { jobId?: string; requestId?: string }): JobResponse {
  let job: JobInfo | undefined;
  if (response.outcome === 'accepted') {
    requireState(Number.isSafeInteger(response.pollAfterMs) && response.pollAfterMs >= 0, 'Invalid poll delay');
    job = response.job;
    requireState(job.state === 'queued' || job.state === 'running', 'Accepted job is terminal');
  } else if (response.outcome === 'succeeded') {
    requireState(response.result.kind === 'job', 'Expected a job response');
    job = response.result.job;
    requireState(job.state === 'succeeded' && job.result?.outcome === 'succeeded', 'Missing terminal receipt');
  } else {
    job = response.job ?? undefined;
    if (job) requireState(job.state === 'failed' || job.state === 'cancelled', 'Failure has a nonterminal job');
  }
  if (job) {
    if (expected.jobId !== undefined) requireState(job.id === expected.jobId, 'Job identity differs');
    if (expected.requestId !== undefined) requireState(job.requestId === expected.requestId, 'Business request identity differs');
  }
  return response as JobResponse;
}

/** Thin control client. The injected host is the only job and commit owner. */
export class OfficeClient {
  constructor(readonly port: OperationPort) {}

  async dispatch(request: HostRequest, options: CallOptions = {}): Promise<HostResponse> {
    checkAbort(options.signal);
    return waiting(this.port.dispatch(request), options.signal);
  }

  async result<K extends HostResult['kind']>(request: HostRequest, kind: K,
    options: CallOptions = {}): Promise<Extract<HostResult, { kind: K }>> {
    const response = await this.dispatch(request, options);
    if (response.outcome === 'failed') throw new ClientError('HOST_FAILURE', response.error.message, {}, response.error);
    requireState(response.outcome === 'succeeded' && response.result.kind === kind, `Expected ${kind} result`);
    return response.result as Extract<HostResult, { kind: K }>;
  }

  async submit(request: OperationRequest, options: CallOptions = {}): Promise<JobResponse> {
    const requestId = request.requestId;
    try {
      return jobResponse(await this.dispatch({ operation: 'submit', request }, options), { requestId });
    } catch (error) {
      if (error instanceof ClientError && error.code === 'WAIT_ABORTED')
        throw new ClientError(error.code, error.message, { requestId });
      throw error;
    }
  }

  async getJob(jobId: string, options: CallOptions = {}): Promise<JobResponse> {
    return jobResponse(await this.dispatch({ operation: 'getJob', jobId }, options), { jobId });
  }

  /** Explicit business cancellation, distinct from CallOptions.signal. */
  async cancelJob(jobId: string, options: CallOptions = {}): Promise<JobResponse> {
    return jobResponse(await this.dispatch({ operation: 'cancelJob', jobId }, options), { jobId });
  }

  async waitForJob(jobId: string, options: WaitOptions = {}): Promise<JobResponse> {
    const timeout = options.timeoutMs ?? 60_000;
    positiveBound(timeout, 2_147_483_647, 'wait timeout');
    const controller = new AbortController();
    const cancel = () => controller.abort();
    if (options.signal?.aborted)
      throw new ClientError('WAIT_ABORTED', 'Local job wait ended; query the same durable job', { jobId });
    options.signal?.addEventListener('abort', cancel, { once: true });
    let timedOut = false;
    const timer = setTimeout(() => { timedOut = true; controller.abort(); }, timeout);
    try {
      for (;;) {
        const response = await this.getJob(jobId, { signal: controller.signal });
        if (response.outcome !== 'accepted') return response;
        // Honor the host hint. Long hints remain interruptible by the budget;
        // do not accelerate polling or create a second background scheduler.
        await delay(Math.max(10, Math.min(response.pollAfterMs, 2_147_483_647)), controller.signal);
      }
    } catch (error) {
      if (error instanceof ClientError && error.code === 'WAIT_ABORTED')
        throw new ClientError(timedOut ? 'WAIT_TIMEOUT' : 'WAIT_ABORTED',
          'Local job wait ended; query the same durable job', { jobId });
      throw error;
    } finally {
      clearTimeout(timer);
      options.signal?.removeEventListener('abort', cancel);
    }
  }
}
