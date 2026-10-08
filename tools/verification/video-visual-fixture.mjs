// Authored verification fixture, not a production media source.
const t = n => ({ ticks: String(n), timescale: 1 });
const r = (a, b) => ({ start: t(a), end: t(b) });
export const ratio = n => ({ numerator: String(n), denominator: '1' });
const map = (a, b, n) => ({ anchors: [{ local: t(0), source: t(a) }, { local: t(n), source: t(b) }] });
const policy = { rate: 'resample', reverse: 'reverse', hold: 'silence' };
const track = (id, kind, clips) => ({ id, name: id, kind, clips, muted: false, solo: false, locked: false });
const clip = (id, source, effects = []) => ({ id, name: id, timeline: r(0, 4), source, effects, audio: policy });
const transform = { kind: 'transform', xMilliPixels: 1230, yMilliPixels: -2370, scaleXPpm: -730000, scaleYPpm: 910000, rotationMilliDegrees: 23000, opacityPpm: 750000 };
export const document = {
  format: 'musteroffice.video/1', id: 'film', title: 'Visual binary parity', canvas: { width: 120, height: 80 },
  frameRate: { numerator: 30000, denominator: 1001 }, audioSampleRate: 48000, rootSequence: 'main', linkGroups: {},
  sequences: { main: { id: 'main', tracks: ['bg', 'nested', 'overlay'] }, child: { id: 'child', tracks: ['source'] } },
  tracks: { bg: track('bg', 'graphics', ['background']), nested: track('nested', 'video', ['group']), overlay: track('overlay', 'graphics', ['still']), source: track('source', 'video', ['video']) },
  clips: {
    background: clip('background', { kind: 'solid', rgba: [15, 23, 42, 255] }),
    group: clip('group', { kind: 'sequence', sequence: 'child', timeMap: map(4, 0, 4) }, [transform]),
    video: clip('video', { kind: 'media', resource: 'movie', stream: 'v', timeMap: map(0, 4, 4) }, [{ ...transform, rotationMilliDegrees: -47000, scaleXPpm: 830000 }]),
    still: clip('still', { kind: 'media', resource: 'image', stream: 'i', timeMap: map(0, 0, 4) }, [{ ...transform, rotationMilliDegrees: 90000 }]),
  },
  resources: {
    movie: { id: 'movie', digest: '13'.repeat(32), byteLength: '100', descriptor: { kind: 'media', streams: { v: { kind: 'video', available: r(0, 4), dimensions: { width: 3, height: 2 }, frameRate: null } } } },
    image: { id: 'image', digest: '35'.repeat(32), byteLength: '100', descriptor: { kind: 'media', streams: { i: { kind: 'image', dimensions: { width: 2, height: 3 } } } } },
  },
};
export const queries = ['0', '1', '29', '59', '60', '119'].map((frame, i) => ({ frame, width: i % 2 ? 24 : 12, height: i % 3 ? 8 : 16, replies: null }));
