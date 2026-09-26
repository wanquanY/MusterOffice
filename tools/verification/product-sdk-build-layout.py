"""Check product source-only Cargo consumption, without copying private sources out."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
product = args.product_repo.resolve()
stage = product / '.codex-work'
stage.mkdir(exist_ok=True)
names = subprocess.check_output([
    'git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z',
    'apps/agent-runtime', 'packages/presentation-engine/contracts',
], cwd=product).decode().split('\0')
names = sorted(set(n for n in names if n and (product / n).is_file()))
assert any('/vendor/musteroffice/' in n for n in names)
assert not any('/target/' in n or n.startswith('components/musteroffice/generated/') for n in names)
source_records = []
with tempfile.TemporaryDirectory(prefix='office-sdk-source-build-', dir=stage) as temporary:
    copy = Path(temporary)
    for name in names:
        source = product / name
        assert not source.is_symlink(), name
        destination = copy / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        data = source.read_bytes()
        source_records.append(dict(path=name, byteLength=len(data), sha256=hashlib.sha256(data).hexdigest()))
    workspace = copy / 'apps/agent-runtime'
    # Artifacts and the shared registry cache are build dependencies, not SDK
    # sources. No generated SDK/worker, source core checkout or private state
    # is in this product source-only fixture.
    environment = dict(os.environ, CARGO_TARGET_DIR=str(product / 'apps/agent-runtime/target'))
    metadata = json.loads(subprocess.check_output([
        'cargo', 'metadata', '--locked', '--offline', '--format-version=1',
    ], cwd=workspace, env=environment))
    packages = {p['id']: p for p in metadata['packages']}
    sdk = [p for p in metadata['packages'] if p['name'].startswith('mo-')]
    assert len(sdk) == 22
    assert all(Path(p['manifest_path']).is_relative_to(workspace / 'vendor/musteroffice') for p in sdk)
    assert not any(packages[k]['name'].startswith('mo-') for k in metadata['workspace_members'])
    assert all(Path(p['manifest_path']).is_relative_to(copy) for p in metadata['packages'] if p['source'] is None)
    process = subprocess.run([
        'cargo', 'check', '-p', 'musterwork-agent-musteroffice', '--lib', '--locked', '--offline',
    ], cwd=workspace, env=environment, capture_output=True, text=True)
    # Keep private build output in the product; publish only hashes and scope.
    log = stage / 'office-sdk-source-build.log'
    log.write_text(process.stdout + process.stderr)
    assert process.returncode == 0, f'source-only build failed; inspect {log}'
    assert 'Finished `dev` profile' in process.stderr
    log_bytes = log.read_bytes()
    result = dict(
        format='musteroffice.product-sdk-source-build/1',
        sourceFiles=source_records, sdkLibraries=22,
        workspaceMembers=len(metadata['workspace_members']),
        noIgnoredSdkOrWorker=True, noCoreCheckoutPath=True, nativeAdapterLibraryCompiled=True,
        sharedRegistryAndBuildCache=True,
        log=dict(path=log.relative_to(product).as_posix(), byteLength=len(log_bytes),
                 sha256=hashlib.sha256(log_bytes).hexdigest()),
        limits='Host macOS compilation; not a container build, fresh registry cache, product execution or platform acceptance.',
    )
args.output.write_text(json.dumps(result, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(files=len(names), sdkLibraries=22, compiled=True)))
