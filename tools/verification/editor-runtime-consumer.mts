import {createPlaybackRuntime, EditorComputationError, type SnapshotRecord, type EditorPagePreparation, type PlaybackCode} from '../../.codex-work/editor-runtime/playback-sdk/index.mjs';
async function consume(modules: PlaybackCode, saved: SnapshotRecord, request: EditorPagePreparation, material: Uint8Array, fonts: Uint8Array) {
  const runtime = await createPlaybackRuntime(modules);
  const snapshot: SnapshotRecord = runtime.editor.restore(saved);
  if (request.input.kind === 'pptx') throw Error('Editable records use author or retained inputs');
  const page = runtime.createEditorPage();
  try {
    const frame = page.prepare({...request,input:{...request.input,document:snapshot.document}}, {
      material,fonts,decoder:runtime.raster,shaping:runtime.shaping,raster:runtime.raster,
    });
    page.query([]); page.pick([],runtime.raster);
    return {view:frame.view,revision:snapshot.revision,pixels:frame.pixels};
  } catch (error) {
    if (error instanceof EditorComputationError) return error.diagnostic.code;
    throw error;
  } finally {page.close();}
}
void consume;
