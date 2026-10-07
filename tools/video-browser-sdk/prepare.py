"""Prepare reviewed upstream source from an explicitly supplied SHA-pinned archive.

No network, installed-package mutation or third-party build scripts. Fetch the
sourceArchive URL in component.json with a normal TLS client before invocation.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import tarfile

ROOT = Path(__file__).resolve().parents[2]


def prepare(archive_path):
    dependency = json.loads((ROOT / "components/video-browser-media/component.json").read_text())
    data = archive_path.read_bytes()
    if hashlib.sha256(data).hexdigest() != dependency["sourceArchiveSha256"]:
        raise ValueError("source archive differs from reviewed pin")
    output = ROOT / ".codex-work/components/mediabunny" / dependency["sourceCommit"]
    files = set(dependency["sourceFiles"])
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        records = {}
        for member in archive.getmembers():
            p = PurePosixPath(member.name)
            if p.is_absolute() or ".." in p.parts:
                raise ValueError("invalid archive member")
            relative = PurePosixPath(*p.parts[1:]).as_posix()
            if relative not in files:
                continue
            if not member.isfile() or relative in records:
                raise ValueError("unique regular source file required")
            records[relative] = archive.extractfile(member).read()
    if records.keys() != files:
        raise ValueError("incomplete corresponding source")
    digest = hashlib.sha256()
    for name in sorted(records):
        digest.update((name + "\0").encode())
        digest.update(records[name])
        digest.update(b"\0")
    if digest.hexdigest() != dependency["sourceTreeSha256"]:
        raise ValueError("source tree differs from reviewed pin")
    if output.exists():
        if output.is_symlink() or any((output / name).is_symlink() or (output / name).read_bytes() != data for name, data in records.items()):
            raise ValueError("existing source cache changed")
    else:
        output.mkdir(parents=True, exist_ok=False)
        for name, body in records.items():
            path = output / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
    return {"status": "prepared", "sourceCommit": dependency["sourceCommit"], "files": len(records)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(args.archive)))
