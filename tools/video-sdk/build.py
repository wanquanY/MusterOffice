"""Build and inventory the independent video authoring SDK for local host integration.

Offline build, explicit wasm-bindgen, new output directory and an external manifest
pin. This does not publish a release or authorize installation in another product.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
FORMAT = "musteroffice.video-authoring-sdk/1-draft"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command):
    return subprocess.check_output(command, cwd=ROOT, text=True)


def inventory(root, exclude=()):
    rows = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise ValueError("SDK inputs and outputs cannot contain symlinks")
        if path.is_dir() or path.relative_to(root).as_posix() in exclude:
            continue
        if not path.is_file():
            raise ValueError("regular SDK file required")
        rows.append({"path": path.relative_to(root).as_posix(), "byteLength": path.stat().st_size, "sha256": sha(path)})
    return rows


def source_inputs():
    tree = run(["cargo", "tree", "--locked", "--offline", "-p", "mo-video-wasm", "--target", "wasm32-unknown-unknown", "--edges", "normal", "--prefix", "none", "--format", "{p}"])
    names = {line.split()[0] for line in tree.splitlines() if line.startswith("mo-")}
    if any(name.startswith(("mo-presentation", "mo-pptx", "mo-media", "mo-kernel")) for name in names):
        raise ValueError("authoring SDK acquired unrelated presentation/media dependencies")
    paths = {ROOT / "Cargo.toml", ROOT / "Cargo.lock"}
    for name in names:
        directory = ROOT / "crates" / name
        paths.add(directory / "Cargo.toml")
        paths.update((directory / "src").rglob("*.rs"))
    paths.update((ROOT / "packages/video-client/src").glob("*.ts"))
    paths.add(ROOT / "packages/video-client/tsconfig.json")
    paths.update((ROOT / "packages/contracts/src/generated").glob("video-*.ts"))
    paths.update((ROOT / "tools/video-sdk").glob("*.py"))
    for path in paths:
        if path.is_symlink() or not path.is_file():
            raise ValueError("invalid source input")
    return sorted(paths), tree


def record(paths):
    return [{"path": p.relative_to(ROOT).as_posix(), "sha256": sha(p), "byteLength": p.stat().st_size} for p in paths]


def verify(root, pin):
    path = root / "bundle-manifest.json"
    if root.is_symlink() or path.is_symlink() or not path.is_file() or path.stat().st_size > 1 << 20 or sha(path) != pin:
        raise ValueError("SDK manifest differs from external pin")
    manifest = json.loads(path.read_text())
    if manifest["format"] != FORMAT or manifest["releaseCleared"] is not False:
        raise ValueError("unsupported development SDK")
    if manifest["files"] != inventory(root, ("bundle-manifest.json",)):
        raise ValueError("SDK file inventory differs")
    return manifest


def build(output, bindgen):
    output = output.resolve()
    for name in ["crates", "tools", "packages", "components", "contracts"]:
        if output.is_relative_to(ROOT / name):
            raise ValueError("output must not modify source directories")
    paths, tree = source_inputs()
    before = record(paths)
    output.mkdir(parents=True, exist_ok=False)
    subprocess.run(["cargo", "build", "--offline", "--locked", "--release", "-p", "mo-video-wasm", "--target", "wasm32-unknown-unknown"], cwd=ROOT, check=True)
    subprocess.run([bindgen, str(ROOT / "target/wasm32-unknown-unknown/release/mo_video_wasm.wasm"), "--target", "web", "--out-dir", str(output / "runtime")], cwd=ROOT, check=True)
    subprocess.run(["pnpm", "exec", "tsc", "--project", "packages/video-client/tsconfig.json", "--outDir", str(output / "lib")], cwd=ROOT, check=True)
    if record(paths) != before:
        raise ValueError("source changed while compiling SDK")
    (output / "index.mjs").write_text('''import initialize, * as kernel from './runtime/mo_video_wasm.js';
import {WasmVideo} from './lib/video-client/src/index.js';
export * from './lib/video-client/src/index.js';
let claimed=false;
export async function createVideoRuntime(module) {
  if(claimed) throw new Error('Video runtime already initialized; use a new host Worker');
  claimed=true;
  if(!(module instanceof WebAssembly.Module)) throw new TypeError('Expected an explicit compiled video module');
  await initialize({module_or_path:module});
  return new WasmVideo(kernel);
}
''')
    (output / "index.d.mts").write_text("export * from './lib/video-client/src/index.js';\nimport type {WasmVideo} from './lib/video-client/src/index.js';\nexport declare function createVideoRuntime(module:WebAssembly.Module):Promise<WasmVideo>;\n")
    package = {"name": "@musteroffice/video-authoring", "version": "0.1.0-video.dev", "private": True, "type": "module", "exports": {".": {"types": "./index.d.mts", "import": "./index.mjs"}}}
    (output / "package.json").write_text(json.dumps(package, indent=2) + "\n")
    for name in ["LICENSE", "NOTICE"]:
        path = ROOT / name
        if path.is_file():
            shutil.copyfile(path, output / name)
    (output / "dependency-tree.txt").write_text(tree)
    (output / "source-inputs.json").write_text(json.dumps(before, indent=2) + "\n")
    manifest = {"format": FORMAT, "releaseCleared": False, "profile": "musteroffice.video.operations/1-draft",
                "sourceCommit": run(["git", "rev-parse", "HEAD"]).strip(), "sourceDirty": bool(run(["git", "status", "--porcelain"])),
                "rustc": run(["rustc", "--version"]).strip(), "wasmBindgen": run([bindgen, "--version"]).strip(),
                "buildHost": platform.platform(), "files": inventory(output),
                "scope": "Local development authoring component. No codec, playback, storage, UI or product acceptance. Formal release/SBOM clearance remains separate."}
    (output / "bundle-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    pin = sha(output / "bundle-manifest.json")
    verify(output, pin)
    return {"status": "built", "manifestSha256": pin, "files": len(manifest["files"]), "bytes": sum(row["byteLength"] for row in manifest["files"])}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path)
    p.add_argument("--bindgen")
    p.add_argument("--verify", type=Path)
    p.add_argument("--sha256")
    args = p.parse_args()
    if args.verify:
        manifest = verify(args.verify, args.sha256)
        print(json.dumps({"status": "verified", "files": len(manifest["files"])}))
    else:
        if not args.output or not args.bindgen:
            p.error("--output and --bindgen required")
        print(json.dumps(build(args.output, args.bindgen)))


if __name__ == "__main__":
    main()
