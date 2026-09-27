/** Runs inside the product's dedicated module Worker, never on the UI thread. */
import {createPlaybackRuntime} from '../index.mjs';
import {createDispatcher, failure} from './dispatch.mjs';

// JS imports are host delivery. Computation cannot silently fetch code/assets.
globalThis.fetch = () => {throw Error('Implicit fetch denied in example Worker');};
let runtime, dispatch, claimed = false, initializing = false;
globalThis.onmessage = async ({data: {id, command: q}}) => {
  try {
    let value;
    if (q.operation === 'initialize') {
      if (claimed) throw Error('Worker already initialized');
      claimed = true; initializing = true;
      runtime = await createPlaybackRuntime(q.modules);
      dispatch = createDispatcher(runtime); initializing = false;
      value = {ready: true};
    } else {
      if (!dispatch || initializing) throw Error('Worker not initialized');
      value = dispatch(q);
    }
    globalThis.postMessage({id, ok: true, value}, value?.pixels ? [value.pixels.buffer] : []);
  } catch (error) {
    globalThis.postMessage(failure(id, error, !dispatch || initializing
      || runtime.raster.invalid || runtime.shaping.invalid));
  }
};
