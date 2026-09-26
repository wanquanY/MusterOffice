"""Record the exact additional host dependencies, archives and license notices."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

out = Path('components/sqlite-host')
out.mkdir(exist_ok=True)
notices = out/'licenses'
notices.mkdir(exist_ok=True)
cargo = Path(os.environ.get('CARGO_HOME', str(Path.home()/'.cargo')))
lock = tomllib.loads(Path('Cargo.lock').read_text())
packages = {p['name']: p for p in lock['package']}
names = ['rusqlite', 'libsqlite3-sys', 'cc', 'fallible-iterator', 'fallible-streaming-iterator', 'find-msvc-tools', 'pkg-config', 'shlex', 'smallvec', 'vcpkg']
records = []
for name, args in [('host', ['-p', 'mo-host']), ('wasm', ['-p', 'mo-wasm', '--target', 'wasm32-unknown-unknown'])]:
    result = subprocess.run(['cargo', 'tree', *args, '--edges', 'normal,build', '--locked', '--offline', '--format', '{p} | {l} | {f}'], check=True, capture_output=True, text=True)
    Path(f'.codex-work/operation-host/{name}-dependency-tree.txt').write_text(result.stdout)
def entry(path):
    data = path.read_bytes()
    return {'byteLength': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
for name in names:
    package = packages[name]
    stem = f"{name}-{package['version']}"
    source, = (cargo/'registry/src').glob('*/'+stem)
    archive, = (cargo/'registry/cache').glob('*/'+stem+'.crate')
    assert entry(archive)['sha256'] == package['checksum'], stem
    manifest = tomllib.loads((source/'Cargo.toml').read_text())['package']
    notice = source/('LICENSE' if name in ['rusqlite', 'libsqlite3-sys'] else 'LICENSE-MIT')
    assert 'Permission is hereby granted' in notice.read_text(), stem
    target = notices/(stem+'.txt')
    shutil.copyfile(notice, target)
    records.append({'name':name, 'version':package['version'], 'archiveUrl':f'https://static.crates.io/crates/{name}/{stem}.crate', **entry(archive), 'declaredLicense':manifest['license'], 'selectedLicense':'MIT', 'notice': {'path':str(target), **entry(target)}})
source, = (cargo/'registry/src').glob('*/libsqlite3-sys-'+packages['libsqlite3-sys']['version'])
amalgamation = source/'sqlite3/sqlite3.c'
code = amalgamation.read_text()
version, = re.findall(r'^#define SQLITE_VERSION\s+"([^"]+)"', code, flags=re.M)
assert version == '3.53.2'
assert 'The author disclaims copyright to this source code.' in code
assert 'rusqlite' not in Path('.codex-work/operation-host/wasm-dependency-tree.txt').read_text()
assert 'libsqlite3-sys' not in Path('.codex-work/operation-host/wasm-dependency-tree.txt').read_text()
record = {'format':'musteroffice.sqlite-host-component/1-draft', 'status':'implementation experiment under ADR 0006; not a release approval', 'integration':{'owner':'mo-standard-host only', 'pureService':'mo-operation-service has no SQLite dependency', 'rusqliteDefaultFeatures':False, 'rusqliteFeatures':['bundled'], 'sqliteSourceUnmodified':True, 'sqlcipher':False, 'publicSqlOrExtensionLoading':False}, 'sqlite':{'version':version, 'license':'public domain declaration in the amalgamation', 'copyrightReference':'https://www.sqlite.org/copyright.html', 'source':{'pathInCrate':'sqlite3/sqlite3.c', **entry(amalgamation)}, 'header':{'pathInCrate':'sqlite3/sqlite3.h', **entry(source/'sqlite3/sqlite3.h')}}, 'additionalLockedPackages':records, 'limitations':['The complete build enables upstream bundled SQLite features; no minimal-size configuration claim.', 'Not a complete release SBOM. Existing dependencies remain in Cargo.lock and their component records.', 'No SQLite or rusqlite added to the production WASM kernel dependency graph.', 'Only current native test environment validated; cross-platform package closure is not yet accepted.']}
(out/'component.json').write_text(json.dumps(record, indent=2)+'\n')
print(json.dumps({'sqlite':version, 'additionalPackages':len(records), 'wasmGraphExcludesSqlite':True}))
