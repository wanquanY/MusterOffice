"""Build/test a Linux x64 worker from a clean commit using an existing local builder.

Archive directories are explicit trusted inputs, verified against component
locks before Docker receives them. No global Docker setting or registry is changed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--builder", required=True)
    parser.add_argument("--archives", type=Path, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--build-proxy", help="Optional proxy reachable from build containers")
    args = parser.parse_args()
    if command("git", "status", "--porcelain"):
        raise RuntimeError("Linux release build requires a clean committed source tree")
    driver = command("docker", "buildx", "inspect", args.builder, "--format", "{{.Driver}}")
    if driver != "docker-container":
        raise RuntimeError("Select an existing local docker-container builder; production Kubernetes is not a builder")
    output = args.output.resolve()
    if output.exists():
        raise RuntimeError("Output must be a new immutable directory")
    expected = []
    for component, suffix in [("skia", "tar.gz"), ("harfbuzz", "tar.xz")]:
        lock = json.loads((ROOT / "components" / component / "lock.json").read_text())
        expected.append({**lock, "archive": f"{component}-{lock.get('commit', lock.get('version'))}.{suffix}"})
    expected += json.loads((ROOT / "components/image-codec/lock.json").read_text())["dependencies"]
    revision = command("git", "rev-parse", "HEAD")
    with tempfile.TemporaryDirectory(prefix="musteroffice-linux-inputs-") as temporary:
        staging = Path(temporary)
        archives = staging / "archives"
        archives.mkdir()
        for entry in expected:
            sources = [directory / entry["archive"] for directory in args.archives
                       if (directory / entry["archive"]).is_file()]
            if not sources:
                raise RuntimeError(f"Missing pinned archive: {entry['archive']}")
            source = sources[0]
            with source.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            if digest != entry["sha256"] or source.stat().st_size != entry["byteLength"]:
                raise RuntimeError(f"Component archive pin mismatch: {entry['archive']}")
            shutil.copyfile(source, archives / entry["archive"])
        snapshot = staging / "source.tar"
        subprocess.run(["git", "archive", "--format=tar", "--output", str(snapshot), revision], cwd=ROOT, check=True)
        build = ["docker", "buildx", "build", "--builder", args.builder, "--platform", "linux/amd64",
                 "--file", "tools/release/linux-worker.Dockerfile", "--target", "artifact",
                 "--build-context", "component-archives=" + str(archives),
                 "--output", "type=local,dest=" + str(output), "--progress", "plain"]
        if args.build_proxy:
            build += ["--build-arg", "HTTP_PROXY=" + args.build_proxy,
                      "--build-arg", "HTTPS_PROXY=" + args.build_proxy]
        with snapshot.open("rb") as source:
            subprocess.run([*build, "-"], cwd=ROOT, stdin=source, check=True)
    report = json.loads((output / "build.json").read_text())
    with (output / "mo-export-worker").open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != report["worker"]["sha256"]:
            raise RuntimeError("Exported worker differs from build evidence")
    (output / "source.json").write_text(json.dumps({"sourceRevision": revision,
        "dockerfileSha256": hashlib.sha256((ROOT / "tools/release/linux-worker.Dockerfile").read_bytes()).hexdigest(),
        "buildReportSha256": hashlib.sha256((output / "build.json").read_bytes()).hexdigest()}, indent=2) + "\n")
    print(json.dumps({"sourceRevision": revision, "directory": str(output), "worker": report["worker"]}))


if __name__ == "__main__":
    main()
