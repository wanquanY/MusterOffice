/** Independent offline Browser media delivery. No Rust/PPT rebuild or publishing. */
import { readFile, writeFile, readdir, mkdir, copyFile, lstat, unlink } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';
import { build as bundle } from 'esbuild';
const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const FORMAT = 'musteroffice.video-browser-media-sdk/1-draft';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
async function walk(root) {
  const files = [];
  async function visit(directory) {
    for (const entry of await readdir(path.join(root, directory), { withFileTypes: true })) {
      assert(!entry.isSymbolicLink(), 'symbolic links are not component files');
      const name = path.join(directory, entry.name);
      if (entry.isDirectory()) await visit(name);
      else { assert(entry.isFile(), 'regular component files required'); files.push(name); }
    }
  }
  await visit(''); return files.sort();
}
async function inventory(root) {
  return Promise.all((await walk(root)).filter(name => name !== 'bundle-manifest.json').map(async name => {
    const bytes = await readFile(path.join(root, name)); return { path: name, byteLength: bytes.length, sha256: sha(bytes) };
  }));
}
async function copy(source, target) { await mkdir(path.dirname(target), { recursive: true }); await copyFile(source, target); }
async function treeDigest(root, files) {
  const hash = createHash('sha256');
  for (const name of [...files].sort()) {
    assert((await lstat(path.join(root, name))).isFile(), 'regular upstream source files required');
    hash.update(name + '\0').update(await readFile(path.join(root, name))).update('\0');
  }
  return hash.digest('hex');
}
export async function verify(directory, pin) {
  assert(!((await lstat(directory)).isSymbolicLink()), 'root must not be a symbolic link');
  const bytes = await readFile(path.join(directory, 'bundle-manifest.json'));
  assert.equal(sha(bytes), pin, 'manifest differs from external pin');
  const manifest = JSON.parse(bytes);
  assert.equal(manifest.format, FORMAT); assert.equal(manifest.releaseCleared, false);
  assert.deepEqual(await inventory(directory), manifest.files, 'component file inventory changed');
  return manifest;
}
export async function build(output) {
  output = path.resolve(output);
  assert(output.startsWith(path.join(ROOT, '.codex-work') + path.sep) || !output.startsWith(ROOT), 'output must not replace source files');
  const dependency = JSON.parse(await readFile(path.join(ROOT, 'components/video-browser-media/component.json')));
  const vendor = path.join(ROOT, '.codex-work/components/mediabunny', dependency.sourceCommit);
  assert.equal(await treeDigest(vendor, dependency.sourceFiles), dependency.sourceTreeSha256, 'dependency source differs from reviewed pin');
  assert.equal(JSON.parse(await readFile(path.join(vendor, 'package.json'))).version, dependency.version);
  const sourcePaths = [
    'package.json', 'pnpm-lock.yaml', 'components/video-browser-media/component.json',
    'packages/video-browser-media/tsconfig.json', 'packages/contracts/src/generated/video-frame-plan.ts',
    ...(await walk(path.join(ROOT, 'packages/video-browser-media/src'))).map(x => 'packages/video-browser-media/src/' + x),
    'tools/video-browser-sdk/build.mjs', 'tools/video-browser-sdk/prepare.py',
  ].sort();
  const source = async () => Promise.all(sourcePaths.map(async name => ({ path: name, sha256: sha(await readFile(path.join(ROOT, name))) })));
  const inputs = await source();
  await mkdir(output, { recursive: false });
  const compiled = path.join(output, 'types');
  execFileSync(path.join(ROOT, 'node_modules/.bin/tsc'), ['--project', 'packages/video-browser-media/tsconfig.json', '--emitDeclarationOnly', '--outDir', compiled], { cwd: ROOT, stdio: 'inherit' });
  for (const name of ['probe.d.ts', 'samples.d.ts', 'time.d.ts']) await unlink(path.join(compiled, 'video-browser-media/src', name));
  await bundle({ absWorkingDir: ROOT, entryPoints: ['packages/video-browser-media/src/index.ts'],
    bundle: true, format: 'esm', platform: 'browser', minify: true, legalComments: 'external', outfile: path.join(output, 'index.mjs'),
    plugins: [{ name: 'reviewed-mediabunny-source', setup(build) {
      build.onResolve({ filter: /^mediabunny$/ }, () => ({ path: path.join(vendor, 'src/index.ts') }));
    } }],
  });
  await writeFile(path.join(output, 'index.d.mts'), "export * from './types/video-browser-media/src/index.js';\n");
  await writeFile(path.join(output, 'package.json'), JSON.stringify({ name: '@musteroffice/video-browser-media', version: '0.1.0-video.dev', private: true, type: 'module', exports: { '.': { types: './index.d.mts', import: './index.mjs' } } }, null, 2) + '\n');
  for (const name of ['LICENSE', 'NOTICE']) await copy(path.join(ROOT, name), path.join(output, name));
  for (const name of dependency.sourceFiles) await copy(path.join(vendor, name), path.join(output, 'third-party/mediabunny', name));
  assert.equal(await treeDigest(vendor, dependency.sourceFiles), dependency.sourceTreeSha256, 'upstream changed during build');
  assert.equal(await treeDigest(path.join(output, 'third-party/mediabunny'), dependency.sourceFiles), dependency.sourceTreeSha256, 'corresponding source changed during copy');
  await writeFile(path.join(output, 'third-party/README.md'), `Mediabunny ${dependency.version}, MPL-2.0, unmodified.\n\nReadable corresponding source and license are included in mediabunny/.\nUpstream: https://github.com/Vanilagy/mediabunny\nPinned npm source: ${dependency.tarball}\n\nMusterOffice original adapter sources remain Apache-2.0. This development bundle is not cleared for public release.\n`);
  await writeFile(path.join(output, 'source-inputs.json'), JSON.stringify(inputs, null, 2) + '\n');
  await writeFile(path.join(output, 'dependency.json'), JSON.stringify(dependency, null, 2) + '\n');
  assert.deepEqual(await source(), inputs, 'sources changed during build');
  const manifest = { format: FORMAT, releaseCleared: false, dependency: { name: dependency.name, version: dependency.version, license: dependency.license, sourceTreeSha256: dependency.sourceTreeSha256 },
    sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim(),
    sourceDirty: Boolean(execFileSync('git', ['status', '--porcelain'], { cwd: ROOT, encoding: 'utf8' }).trim()),
    files: await inventory(output), scope: 'Browser media input only. Product owns byte admission and Worker lifecycle. Not a renderer/exporter or a cleared public release.' };
  const bytes = JSON.stringify(manifest, null, 2) + '\n';
  await writeFile(path.join(output, 'bundle-manifest.json'), bytes);
  const pin = sha(bytes); await verify(output, pin);
  return { status: 'built', manifestSha256: pin, files: manifest.files.length, bytes: manifest.files.reduce((sum, row) => sum + row.byteLength, 0) };
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [command, directory, pin] = process.argv.slice(2);
  assert(directory && (command === 'build' || command === 'verify'), 'usage: build.mjs build NEW_OUTPUT_DIRECTORY | verify DIRECTORY MANIFEST_SHA256');
  console.log(JSON.stringify(command === 'build' ? await build(directory) : { status: 'verified', files: (await verify(directory, pin)).files.length }));
}
