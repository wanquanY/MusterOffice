"""Build the independent bounded media index WASM. No codecs, PPT, storage or upload."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
FORMAT = "musteroffice.media-index-sdk/1-draft"


def run(args):
    return subprocess.check_output(args, cwd=ROOT, text=True)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(root):
    result = []
    for p in sorted(root.rglob("*")):
        if p.is_symlink():
            raise ValueError("component symlink")
        if p.is_dir() or p.relative_to(root).as_posix() == "bundle-manifest.json":
            continue
        if not p.is_file():
            raise ValueError("regular component file required")
        result.append({"path": p.relative_to(root).as_posix(), "sha256": sha(p), "byteLength": p.stat().st_size})
    return result


def verify(output, pin):
    manifest = output / "bundle-manifest.json"
    if output.is_symlink() or not output.is_dir() or manifest.is_symlink() or not manifest.is_file() or manifest.stat().st_size > 1 << 20 or sha(manifest) != pin:
        raise ValueError("manifest pin mismatch")
    value = json.loads(manifest.read_text())
    if value["format"] != FORMAT or value["releaseCleared"] is not False or value["files"] != inventory(output):
        raise ValueError("invalid media index component")
    return value


def build(output, bindgen):
    output = output.resolve()
    if output.is_relative_to(ROOT) and not output.is_relative_to(ROOT / ".codex-work"):
        raise ValueError("output must not replace repository sources")
    tree = run(["cargo", "tree", "--offline", "--locked", "-p", "mo-media-index-wasm", "--target", "wasm32-unknown-unknown", "--edges", "normal", "--prefix", "none", "--format", "{p}"])
    names = {line.split()[0] for line in tree.splitlines() if line.startswith("mo-")}
    if not names <= {"mo-media-index-wasm", "mo-media-isobmff", "mo-media", "mo-time", "mo-common"}:
        raise ValueError("index component acquired unrelated dependencies")
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__).resolve()]
    for name in sorted(names):
        paths.append(ROOT / "crates" / name / "Cargo.toml")
        paths.extend(sorted((ROOT / "crates" / name / "src").rglob("*.rs")))
    def sources():
        if any(p.is_symlink() or not p.is_file() for p in paths):
            raise ValueError("regular sources required")
        return [{"path": p.relative_to(ROOT).as_posix(), "sha256": sha(p)} for p in paths]
    before = sources()
    output.mkdir(parents=True, exist_ok=False)
    subprocess.run(["cargo", "build", "--offline", "--locked", "--release", "-p", "mo-media-index-wasm", "--target", "wasm32-unknown-unknown"], cwd=ROOT, check=True)
    subprocess.run([bindgen, str(ROOT / "target/wasm32-unknown-unknown/release/mo_media_index_wasm.wasm"), "--target", "web", "--out-dir", str(output / "runtime")], check=True)
    if before != sources():
        raise ValueError("sources changed during compilation")
    (output / "index.mjs").write_text('''import init, {MediaIndex, media_index_profile} from './runtime/mo_media_index_wasm.js';
let claimed = false;
export async function createMediaIndexRuntime(module) {
  if (claimed || !(module instanceof WebAssembly.Module)) throw new TypeError('An explicit module and fresh Worker are required');
  claimed = true;
  await init({module_or_path: module});
  return Object.freeze({MediaIndex, profile: media_index_profile()});
}
''')
    (output / "index.d.mts").write_text("import type {MediaIndex} from './runtime/mo_media_index_wasm.js';\nexport declare function createMediaIndexRuntime(module:WebAssembly.Module):Promise<Readonly<{MediaIndex:typeof MediaIndex;profile:string}>>;\n")
    (output / "package.json").write_text(json.dumps({"name":"@musteroffice/media-index", "version":"0.1.0-video.dev", "private":True,"type":"module","exports":{".":{"types":"./index.d.mts","import":"./index.mjs"}}}, indent=2)+"\n")
    (output / "source-inputs.json").write_text(json.dumps(before, indent=2)+"\n")
    (output / "dependency-tree.txt").write_text(tree)
    for name in ("LICENSE", "NOTICE"):
        shutil.copyfile(ROOT / name, output / name)
    manifest = {"format":FORMAT,"profile":"musteroffice.media-index/1-draft","releaseCleared":False,"sourceCommit":run(["git","rev-parse","HEAD"]).strip(),
                "sourceDirty":bool(run(["git","status","--porcelain"]).strip()), "rustc":run(["rustc","--version"]).strip(),
                "wasmBindgen":run([bindgen,"--version"]).strip(),"files":inventory(output),
                "scope":"Bounded ISO BMFF metadata and exact movie time. Host owns byte identity and Worker termination. No codec, renderer, export or product acceptance."}
    (output / "bundle-manifest.json").write_text(json.dumps(manifest, indent=2)+"\n")
    pin = sha(output / "bundle-manifest.json")
    verify(output, pin)
    return {"status":"built","manifestSha256":pin,"files":len(manifest["files"]),"bytes":sum(f["byteLength"] for f in manifest["files"])}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--bindgen")
    parser.add_argument("--verify", type=Path)
    parser.add_argument("--sha256")
    args = parser.parse_args()
    if args.verify:
        print(json.dumps({"status":"verified","files":len(verify(args.verify,args.sha256)["files"])}))
    elif args.output and args.bindgen:
        print(json.dumps(build(args.output,args.bindgen)))
    else:
        parser.error("--output/--bindgen or --verify/--sha256 required")
