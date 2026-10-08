"""Offline immutable WASM component delivery; never downloads or publishes."""
import argparse
import importlib.util
import json
import shutil
import subprocess
from pathlib import Path

WRAPPER = Path(__file__).resolve()
spec = importlib.util.spec_from_file_location("wasm_media_delivery", WRAPPER.parents[1] / "wasm-media-sdk/delivery.py")
delivery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(delivery)
COMPONENT = {'crate': 'mo-video-visual-wasm', 'format': 'musteroffice.video-visual-sdk/1-draft', 'profile': 'musteroffice.project-visual-worker/1-draft', 'allowed': ['mo-video-visual-wasm', 'mo-video-gpu-plan', 'mo-video-render', 'mo-video-compile', 'mo-video-model', 'mo-media', 'mo-color', 'mo-time', 'mo-common'], 'exports': ['ProjectVisual', 'VisualCandidate', 'composition_shader'], 'factory': 'createVisualRuntime', 'profileFunction': 'visual_profile', 'package': '@musteroffice/video-visual', 'scope': 'Shared immutable project frame compilation, exact resource requests and metadata validation, byte-exact GPU program ABI and shared shader. Host owns actual GPU pixels, color normalization, authorization, decode, device, presentation and Worker lifetime. No text raster, browser end-to-end acceptance or cleared public release.'}
FORMAT = COMPONENT["format"]
inventory, sha = delivery.inventory, delivery.sha

def verify(output, pin):
    return delivery.verify(output, pin, FORMAT)

def build(output, bindgen):
    root = delivery.ROOT
    client = root / "packages/video-browser-gpu"
    dependency = root / "node_modules/@webgpu/types"
    if json.loads((dependency / "package.json").read_text())["version"] != "0.1.74":
        raise ValueError("pinned WebGPU declarations required")
    paths = sorted((client / "src").glob("*.ts")) + [client / "tsconfig.json", root / "package.json", root / "pnpm-lock.yaml", dependency / "package.json", dependency / "dist/index.d.ts", dependency / "LICENSE"]
    before = [{"path": p.relative_to(root).as_posix(), "sha256": sha(p)} for p in paths]
    delivery.build(output, bindgen, COMPONENT, WRAPPER)
    subprocess.run(["pnpm", "exec", "tsc", "--project", str(client / "tsconfig.json"), "--outDir", str(output.resolve() / "client")], cwd=root, check=True)
    for item in before:
        if sha(root / item["path"]) != item["sha256"]:
            raise ValueError("browser sources changed during compilation")
    inputs = output / "source-inputs.json"
    inputs.write_text(json.dumps(json.loads(inputs.read_text()) + before, indent=2) + "\n")
    shutil.copyfile(dependency / "dist/index.d.ts", output / "client/webgpu.d.ts")
    shutil.copyfile(dependency / "LICENSE", output / "client/LICENSE-webgpu-types.txt")
    declarations = output / "client/index.d.ts"
    declarations.write_text('/// <reference path="./webgpu.d.ts" />\n' + declarations.read_text())
    package = output / "package.json"
    value = json.loads(package.read_text())
    value["exports"]["./gpu"] = {"types": "./client/index.d.ts", "import": "./client/index.js"}
    package.write_text(json.dumps(value, indent=2) + "\n")
    manifest = output / "bundle-manifest.json"
    value = json.loads(manifest.read_text())
    for item in json.loads(inputs.read_text()):
        if sha(root / item["path"]) != item["sha256"]:
            raise ValueError("component sources changed during delivery")
    if delivery.run(["git", "rev-parse", "HEAD"]).strip() != value["sourceCommit"]:
        raise ValueError("source commit changed during delivery")
    value["files"] = inventory(output)
    value["browserExecutor"] = {"format": "musteroffice.browser-gpu-executor/1-draft", "typesPackage": "@webgpu/types@0.1.74", "typesLicense": "BSD-3-Clause"}
    manifest.write_text(json.dumps(value, indent=2) + "\n")
    pin = sha(manifest)
    verify(output, pin)
    return {"status": "built", "manifestSha256": pin, "files": len(value["files"]), "bytes": sum(f["byteLength"] for f in value["files"])}

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
