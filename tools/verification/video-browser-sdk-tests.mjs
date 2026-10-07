import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { build, verify } from '../video-browser-sdk/build.mjs';
test('independent media delivery verifies pin, bytes, extra files and symlinks', async () => {
  const temporary = await mkdtemp(path.join(tmpdir(), 'office-browser-sdk-'));
  try {
    const output = path.join(temporary, 'bundle');
    const result = await build(output), pin = result.manifestSha256;
    const manifest = await verify(output, pin);
    assert.equal(manifest.dependency.version, '1.61.3');
    assert(manifest.files.some(x => x.path === 'third-party/mediabunny/LICENSE'));
    assert(manifest.files.some(x => x.path === 'third-party/mediabunny/src/media-sink.ts'));
    const entry = path.join(output, 'index.mjs'), original = await readFile(entry);
    await writeFile(entry, Buffer.concat([original, Buffer.from('\n// changed\n')]));
    await assert.rejects(verify(output, pin), /inventory/);
    await writeFile(entry, original);
    await assert.rejects(verify(output, '0'.repeat(64)), /external pin/);
    await writeFile(path.join(output, 'unexpected.txt'), 'extra');
    await assert.rejects(verify(output, pin), /inventory/);
    await rm(path.join(output, 'unexpected.txt'));
    await symlink(entry, path.join(output, 'unexpected-link'));
    await assert.rejects(verify(output, pin), /symbolic links/);
    await rm(path.join(output, 'unexpected-link'));
    await verify(output, pin);
  } finally { await rm(temporary, { recursive: true, force: true }); }
});
