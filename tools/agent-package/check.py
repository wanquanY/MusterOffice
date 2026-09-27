"""Assemble real native binaries, relocate, and exercise both MCP eras.

Includes a compatibility-manifest launch and stable host config across package
replacement. It does not install into any Agent app or claim release acceptance.
"""
import argparse
from pathlib import Path
import shutil

import build
from check_support import (PackagedSession, compare_exports, extract_owned_archive,
                           negative_startups, workflow)
from compute_support import Files, read, sha, write


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('mcp', type=Path)
    parser.add_argument('worker', type=Path)
    parser.add_argument('--baseline', type=Path, help='earlier independent caller output tree')
    args = parser.parse_args()
    root = args.output.resolve(); root.mkdir(parents=True, exist_ok=False)
    original = root / 'assembly/musteroffice'
    manifest = build.build(original, args.mcp, args.worker)
    archive = root / 'musteroffice.zip'
    archive_sha = build.archive(original, archive)
    package = root / 'relocated with spaces/${PLUGIN_DATA}/musteroffice'
    extract_owned_archive(archive, package)
    assert build.verify(package) == manifest
    # Second independent assembly, identical source/version; simulates a host
    # replacing package paths, not compatibility with a future binary release.
    replacement = root / 'replacement with spaces/musteroffice'
    assert build.build(replacement, args.mcp, args.worker) == manifest
    assert build.archive(replacement, root / 'replacement.zip') == archive_sha
    worker = next((package / 'bin').glob('mo-export-worker*'))
    worker_sha = build.digest(worker)
    data = root / 'client plugin data'; data.mkdir()
    observations = []
    for era in ('2025-11-25', '2026-07-28'):
        files = Files(root / era)
        caller = data / 'caller-files.json'
        # Only this simulated host writes configuration. The native runtime
        # must leave these bytes unchanged through computation/replacement.
        shutil.copyfile(files.config, caller)
        config_sha = build.digest(caller)
        with PackagedSession(package, data, files, era, 'portable') as client:
            outputs = workflow(client, files, worker_sha)
            observations.append(dict(protocol=era, calls=client.seq, exports=outputs))
        assert build.digest(caller) == config_sha
        saved = (files.outputs / 'created/result.json').read_bytes()
        before = build.inventory(files.outputs)
        with PackagedSession(replacement, data, files, era, 'compatibility-replacement', True) as client:
            caps = client.tool('mo_capabilities', {})
            assert caps['exportRenderer']['implementationSha256'] == worker_sha
            response = client.call('resources/read', {'uri': 'musteroffice://output/created/result.json'})['result']['contents'][0]
            assert response['text'].encode() == saved
        assert build.digest(caller) == config_sha and build.inventory(files.outputs) == before
        rejected = negative_startups(files.root, package, worker, caller)
        assert build.digest(caller) == config_sha and build.inventory(files.outputs) == before
        assert sorted(p.name for p in data.iterdir()) == ['caller-files.json']
        files.clean()
        observations[-1].update(rejectedStartups=rejected, replacementConfigUnchanged=True,
                                 replacementOutputsUnchanged=True)
    compare_exports(root / '2025-11-25/output', root / '2026-07-28/output')
    if args.baseline:
        compare_exports(root / '2025-11-25/output', args.baseline)
    assert build.verify(package) == manifest and build.verify(replacement) == manifest
    report = dict(format='musteroffice.agent-package-check/1', status='passed',
                  targetOs=manifest['targetOs'], targetArch=manifest['targetArch'],
                  archiveSha256=archive_sha, archiveByteLength=archive.stat().st_size,
                  archiveReproducible=True, packageFiles=len(manifest['files']),
                  mcpSha256=build.digest(args.mcp), workerSha256=worker_sha,
                  skillSha256=manifest['skillSha256'], protocolOutputsByteIdentical=True,
                  historicalOutputComparison=args.baseline is not None, observations=observations,
                  installedClientAcceptance=False, releaseCleared=False, productCommitted=False,
                  scope='Owned two-page fixture, macOS/Linux pipe client; debug assembly is not complete PPT, Office/WPS, release size or product acceptance.')
    write(root / 'report.json', report)
    print({key: value for key, value in report.items() if key != 'observations'})


if __name__ == '__main__':
    main()
