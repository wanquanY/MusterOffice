import * as ts from 'typescript/unstable/ast';
import { API } from 'typescript/unstable/sync';
import { createVirtualFileSystem } from 'typescript/unstable/fs';
import version from 'typescript';
import assert from 'node:assert/strict';

const banner = '/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */\n';
// Split only at complete declaration boundaries. Public names stay available at
// the original entry module; dependency imports are type-only, including cycles.
export function modules(stem, code) {
  if (code.split('\n').length <= 2000) return new Map([[`${stem}.ts`, code]]);
  // The installed compiler is 7.0.2. Its AST API is explicitly unstable: pin
  // and verify the version instead of assuming the removed pre-7 compiler API.
  assert.equal(version.version, '7.0.2');
  const file='/musteroffice-contracts/input.ts';
  const api=new API({fs:createVirtualFileSystem({[file]:code})});
  let declarations;
  try {
    const snapshot=api.updateSnapshot({openFiles:[file]});
    try {
      const project=snapshot.getDefaultProjectForFile(file);
      assert.ok(project);
      assert.equal(project.program.getSyntacticDiagnostics(file).length,0,'generated TypeScript must parse');
      const source=project.program.getSourceFile(file);
      assert.ok(source);
      declarations = source.statements.map(statement => {
    assert.ok(ts.isInterfaceDeclaration(statement) || ts.isTypeAliasDeclaration(statement), 'only generated type declarations may be split');
    assert.ok(statement.modifiers?.some(m => m.kind === ts.SyntaxKind.ExportKeyword));
    const text = statement.getFullText(source).trim() + '\n';
    assert.ok(text.split('\n').length <= 1200, `single declaration ${statement.name.text} needs a schema refactor`);
    const refs = new Set();
    function visit(n) {
      if (ts.isTypeReferenceNode(n) && ts.isIdentifier(n.typeName)) refs.add(n.typeName.text);
      if (ts.isExpressionWithTypeArguments(n) && ts.isIdentifier(n.expression)) refs.add(n.expression.text);
      n.forEachChild(visit);
    }
    visit(statement);
    return {name: statement.name.text, text, refs};
      });
    } finally {snapshot.dispose();}
  } finally {api.close();}
  const chunks = []; let chunk = [], lines = 0;
  for (const d of declarations) {
    const size = d.text.split('\n').length;
    if (chunk.length && lines + size > 1200) { chunks.push(chunk); chunk = []; lines = 0; }
    chunk.push(d); lines += size;
  }
  if (chunk.length) chunks.push(chunk);
  const owner = new Map();
  chunks.forEach((items, i) => items.forEach(d => { assert.ok(!owner.has(d.name)); owner.set(d.name, i); }));
  const part = i => `part-${String(i + 1).padStart(3, '0')}`;
  const output = new Map();
  output.set(`${stem}.ts`, banner + chunks.map((items, i) => `export type { ${items.map(d => d.name).join(', ')} } from './${stem}/${part(i)}.js';\n`).join(''));
  chunks.forEach((items, i) => {
    const imports = new Map();
    for (const d of items) for (const name of d.refs) {
      const target = owner.get(name);
      if (target === undefined || target === i) continue;
      if (!imports.has(target)) imports.set(target, new Set());
      imports.get(target).add(name);
    }
    const header = [...imports].sort(([a], [b]) => a - b).map(([target, names]) => `import type { ${[...names].sort().join(', ')} } from './${part(target)}.js';\n`).join('');
    const text = banner + header + '\n' + items.map(d => d.text).join('\n');
    assert.ok(text.split('\n').length <= 2000);
    output.set(`${stem}/${part(i)}.ts`, text);
  });
  return output;
}
