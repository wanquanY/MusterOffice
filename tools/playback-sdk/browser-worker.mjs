/** Browser host example, not a computation service or office wire protocol.
 * The product serves immutable trusted JS and supplies compiled WASM explicitly. */
export async function openBrowserWorker(modules, {
  workerUrl = new URL('./browser-runtime.mjs', import.meta.url), timeoutMs = 30000, signal,
} = {}) {
  const {kernel, raster, text} = modules;
  for (const code of [kernel, raster, text]) {
    if (!(code instanceof WebAssembly.Module)) throw TypeError('Explicit compiled modules required');
  }
  if (signal?.aborted) throw new DOMException('Worker cancelled', 'AbortError');
  const host = new BrowserHost(new Worker(workerUrl, {type: 'module', name: 'MusterOffice computation'}));
  try {
    const reply = await host.call({operation: 'initialize', modules: {kernel, raster, text}}, [], timeoutMs, signal);
    if (reply?.ready !== true) throw Error('Worker did not initialize');
    return host;
  } catch (error) {
    try {host.close();} catch (cleanup) {throw new AggregateError([error, cleanup], 'Initialization and cleanup failed');}
    throw error;
  }
}

class BrowserHost {
  #worker; #pending; #next = 0; #closed = false;
  constructor(worker) {
    this.#worker = worker;
    worker.onmessage = ({data: reply}) => {
      if (this.#closed) return;
      const pending = this.#pending;
      if (!pending || !reply || reply.id !== pending.id || typeof reply.ok !== 'boolean') {
        this.#stop(Error('Uncorrelated worker response')); return;
      }
      let error;
      if (!reply.ok) {
        if (typeof reply.error?.message !== 'string' || typeof reply.fatal !== 'boolean') {
          this.#stop(Error('Malformed worker error')); return;
        }
        error = Object.assign(new Error(reply.error.message), reply.error);
        if (reply.fatal) {this.#stop(error); return;}
      }
      this.#pending = undefined; this.#cleanup(pending);
      if (error) pending.reject(error); else pending.resolve(reply.value);
    };
    worker.onerror = event => {event.preventDefault(); this.#stop(Error(event.message || 'Worker failed'));};
    worker.onmessageerror = () => this.#stop(Error('Worker response could not be cloned'));
  }
  get closed() {return this.#closed;}
  #cleanup(pending) {
    clearTimeout(pending.timer);
    pending.signal?.removeEventListener('abort', pending.abort);
  }
  #stop(error) {
    if (this.#closed) return;
    this.#closed = true;
    const pending = this.#pending; this.#pending = undefined;
    if (pending) this.#cleanup(pending);
    const worker = this.#worker; this.#worker = undefined;
    worker.onmessage = worker.onerror = worker.onmessageerror = null;
    // Web Worker termination has no completion Promise or RSS acknowledgement.
    // Browsers can delay forced interruption of a busy script. Do not reuse it.
    try {worker.terminate();}
    catch (cleanup) {
      error = new AggregateError([error, cleanup], 'Worker failure and termination failure');
      if (!pending) throw error;
    }
    pending?.reject(error);
  }
  call(command, transfer = [], timeoutMs = 30000, signal) {
    if (this.#closed) return Promise.reject(Error('Worker closed'));
    if (this.#pending) return Promise.reject(Error('Worker busy'));
    if (!Number.isFinite(timeoutMs) || timeoutMs <= 0 || timeoutMs > 3600000) {
      return Promise.reject(Error('Invalid deadline'));
    }
    if (signal?.aborted) return Promise.reject(new DOMException('Worker cancelled', 'AbortError'));
    if (this.#next === Number.MAX_SAFE_INTEGER) {
      this.#stop(Error('Worker correlation exhausted')); return Promise.reject(Error('Worker closed'));
    }
    const id = ++this.#next;
    return new Promise((resolve, reject) => {
      const abort = () => this.#stop(new DOMException('Worker cancelled', 'AbortError'));
      this.#pending = {id, resolve, reject, signal, abort,
        timer: setTimeout(() => this.#stop(Error('Worker deadline')), timeoutMs)};
      signal?.addEventListener('abort', abort, {once: true});
      try {this.#worker.postMessage({id, command}, transfer);} catch (error) {this.#stop(error);}
    });
  }
  close() {this.#stop(Error('Worker stopped'));}
}
