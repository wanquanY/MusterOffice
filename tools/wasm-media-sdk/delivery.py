"""Common offline delivery for independent, dependency-bounded media WASM."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


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


def verify(output, pin, format):
    manifest = output / "bundle-manifest.json"
    if output.is_symlink() or not output.is_dir() or manifest.is_symlink() or not manifest.is_file() or manifest.stat().st_size > 1 << 20 or sha(manifest) != pin:
        raise ValueError("manifest pin mismatch")
    value = json.loads(manifest.read_text())
    if value["format"] != format or value["releaseCleared"] is not False or value["files"] != inventory(output):
        raise ValueError("invalid media component")
    return value


def build(output, bindgen, component, wrapper):
    output = output.resolve()
    if output.is_relative_to(ROOT) and not output.is_relative_to(ROOT / ".codex-work"):
        raise ValueError("output must not replace repository sources")
    tree = run(["cargo", "tree", "--offline", "--locked", "-p", component["crate"], "--target", "wasm32-unknown-unknown", "--edges", "normal", "--prefix", "none", "--format", "{p}"])
    names = {line.split()[0] for line in tree.splitlines() if line.startswith("mo-")}
    if not names <= set(component["allowed"]):
        raise ValueError("component acquired unrelated dependencies")
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__).resolve(), wrapper]
    for name in sorted(names):
        paths.append(ROOT / "crates" / name / "Cargo.toml")
        paths.extend(sorted((ROOT / "crates" / name / "src").rglob("*.rs")))
    def sources():
        if any(p.is_symlink() or not p.is_file() for p in paths):
            raise ValueError("regular sources required")
        return [{"path": p.relative_to(ROOT).as_posix(), "sha256": sha(p)} for p in paths]
    before = sources()
    output.mkdir(parents=True, exist_ok=False)
    subprocess.run(["cargo", "build", "--offline", "--locked", "--release", "-p", component["crate"], "--target", "wasm32-unknown-unknown"], cwd=ROOT, check=True)
    subprocess.run([bindgen, str(ROOT / ("target/wasm32-unknown-unknown/release/" + component["crate"].replace("-", "_") + ".wasm")), "--target", "web", "--out-dir", str(output / "runtime")], check=True)
    if before != sources():
        raise ValueError("sources changed during compilation")
    stem = component["crate"].replace("-", "_")
    names_js = ", ".join(component["exports"])
    factory = component["factory"]
    profile_function = component["profileFunction"]
    (output / "index.mjs").write_text(f'''import init, {{{names_js}, {profile_function}}} from './runtime/{stem}.js';
let claimed = false;
export async function {factory}(module) {{
  if (claimed || !(module instanceof WebAssembly.Module)) throw new TypeError('An explicit module and fresh Worker are required');
  claimed = true;
  await init({{module_or_path: module}});
  return Object.freeze({{{names_js}, profile: {profile_function}()}});
}}
''')
    types = ";".join(name+":typeof "+name for name in component["exports"])
    (output / "index.d.mts").write_text(f"import type {{{names_js}}} from './runtime/{stem}.js';\nexport declare function {factory}(module:WebAssembly.Module):Promise<Readonly<{{{types};profile:string}}>>;\n")
    (output / "package.json").write_text(json.dumps({"name":component["package"], "version":"0.1.0-video.dev", "private":True,"type":"module","exports":{".":{"types":"./index.d.mts","import":"./index.mjs"}}}, indent=2)+"\n")
    (output / "source-inputs.json").write_text(json.dumps(before, indent=2)+"\n")
    (output / "dependency-tree.txt").write_text(tree)
    for name in ("LICENSE", "NOTICE"):
        shutil.copyfile(ROOT / name, output / name)
    manifest = {"format":component["format"],"profile":component["profile"],"releaseCleared":False,"sourceCommit":run(["git","rev-parse","HEAD"]).strip(),
                "sourceDirty":bool(run(["git","status","--porcelain"]).strip()), "rustc":run(["rustc","--version"]).strip(),
                "wasmBindgen":run([bindgen,"--version"]).strip(),"files":inventory(output),
                "scope":component["scope"]}
    (output / "bundle-manifest.json").write_text(json.dumps(manifest, indent=2)+"\n")
    pin = sha(output / "bundle-manifest.json")
    verify(output, pin, component["format"])
    return {"status":"built","manifestSha256":pin,"files":len(manifest["files"]),"bytes":sum(f["byteLength"] for f in manifest["files"])}
