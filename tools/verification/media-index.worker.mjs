import { createMediaIndexRuntime } from '/sdk/index.mjs';
self.onmessage = async ({data}) => {
  try {
    const { MediaIndex, profile } = await createMediaIndexRuntime(data.module);
    if (data.stall) {
      MediaIndex.open(100n, () => { self.postMessage({enteredRead:true}); for (;;) { /* host must terminate this Worker */ } }, '');
      throw Error('stalled source unexpectedly returned');
    }
    const results = [];
    for (const fixture of data.fixtures) {
      const reader = new FileReaderSync();
      let totalRead = 0, maxRead = 0, calls = 0;
      const readAt = (offset, count) => {
        if (typeof offset !== 'bigint' || !Number.isInteger(count) || count <= 0 || count > 65536) throw new Error('invalid ReadAt request');
        const start = Number(offset), end = start + Math.min(count, 13);
        if (!Number.isSafeInteger(start) || start < 0 || end > fixture.blob.size) throw new Error('read outside blob');
        const bytes = new Uint8Array(reader.readAsArrayBuffer(fixture.blob.slice(start, end)));
        totalRead += bytes.length; maxRead = Math.max(maxRead, count); calls++; return bytes;
      };
      const index = MediaIndex.open(BigInt(fixture.blob.size), readAt, '');
      try {
        const values = fixture.requests.map(request => {
          try {
            if (request.method === 'avcBootstrap') return {ok:true,value:Array.from(index.avcBootstrap(JSON.stringify(request.body)))};
            const text = request.method === 'summary' ? index.summary()
              : request.method === 'videoTiming' ? index.videoTiming(request.body)
              : request.method === 'scanAvc' ? index.scanAvc(JSON.stringify(request.body), readAt)
              : index[request.method](JSON.stringify(request.body));
            return {ok:true,value:JSON.parse(text)};
          } catch(error) { return {ok:false,error:JSON.parse(error)}; }
        });
        results.push({values,totalRead,maxRead,calls});
      } finally { index.free(); }
    }
    const rejected = [];
    for (const callback of [() => new Uint8Array(), (_offset,count) => new Uint8Array(count+1), () => {throw Error('read');}, () => []]) {
      try { MediaIndex.open(100n,callback,'').free(); rejected.push(null); }
      catch(error) { rejected.push(JSON.parse(error).kind); }
    }
    self.postMessage({ok:true,profile,results,rejected});
  } catch(error) { self.postMessage({ok:false,error:String(error),stack:error.stack}); }
};
