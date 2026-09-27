/** Shared receiving-product example. The SDK owns all document semantics. */
export function createDispatcher(runtime) {
  let owner, execution;
  const withExecution = body => {
    if (!execution) throw Error('No active sampled frame');
    try { return body(execution); }
    finally { if (execution.closed) execution = undefined; }
  };
  return q => {
    switch (q.operation) {
      case 'prepare': {
        if (owner) throw Error('Dispose existing owner first');
        if (q.kind !== 'author' && q.kind !== 'source') throw Error('Unknown prepare kind');
        owner = q.kind === 'author' ? runtime.playback.prepareAuthor(q.request)
          : runtime.playback.prepareSource(q.request, {
            source: q.source, fonts: q.fonts, decoder: runtime.raster, shaping: runtime.shaping,
          });
        // Prove that subsequent frames do not depend on retained input arrays.
        if (q.kind === 'source') {
          const buffers = [...new Set([q.source.buffer, q.fonts.buffer])];
          structuredClone(buffers, {transfer: buffers});
        }
        return {info: owner.info, inputsDetached: q.kind === 'source'
          ? q.source.byteLength === 0 && q.fonts.byteLength === 0 : null};
      }
      case 'sample': return owner.sample(q.at, runtime.raster, q.history ?? null);
      case 'beginSample': {
        if (execution) throw Error('Close the active sampled frame first');
        execution = owner.beginSample(q.at, runtime.raster, q.history ?? null);
        return {begun: true};
      }
      case 'stepSample': return withExecution(frame => ({complete: frame.step(q.workUnits ?? 1)}));
      case 'takeSample': return withExecution(frame => frame.take());
      case 'cancelSample': {
        if (execution) withExecution(frame => frame.close());
        return {cancelled: true};
      }
      case 'timing': return owner.timing();
      case 'advance': return owner.advance(q.generation);
      case 'dispose': {
        if (execution) withExecution(frame => frame.close());
        const current = owner; owner = undefined;
        current.dispose(); return {disposed: true};
      }
      default: throw Error('Unknown host example operation');
    }
  };
}

export function failure(id, error, fatal) {
  return {id, ok: false, fatal, error: {name: error.name, message: error.message,
    ...(error.code ? {code: error.code} : {}),
    ...(error.diagnostic ? {diagnostic: error.diagnostic, kind: error.kind} : {})}};
}
