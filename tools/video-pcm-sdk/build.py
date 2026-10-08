"""Offline immutable WASM component delivery; never downloads or publishes."""
import argparse
import importlib.util
import json
from pathlib import Path

WRAPPER = Path(__file__).resolve()
spec = importlib.util.spec_from_file_location("wasm_media_delivery", WRAPPER.parents[1] / "wasm-media-sdk/delivery.py")
delivery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(delivery)
COMPONENT = {'crate': 'mo-video-pcm-wasm', 'format': 'musteroffice.video-pcm-sdk/1-draft', 'profile': 'musteroffice.pcm-worker/1-draft', 'allowed': ['mo-video-pcm-wasm', 'mo-video-pcm', 'mo-video-audio', 'mo-video-compile', 'mo-video-model', 'mo-media-isobmff', 'mo-media', 'mo-time', 'mo-common'], 'exports': ['PcmDecode', 'MoviePcm'], 'factory': 'createPcmRuntime', 'profileFunction': 'pcm_profile', 'package': '@musteroffice/video-pcm', 'scope': 'Checked microsecond PCM transport, EOF-sealed canonical tiles and shared Rust movie edit DSP. Host owns codecs, authorization, storage, atomic publication and Worker termination. No product acceptance or cleared public release.'}
FORMAT = COMPONENT["format"]
inventory, sha = delivery.inventory, delivery.sha

def verify(output, pin):
    return delivery.verify(output, pin, FORMAT)

def build(output, bindgen):
    return delivery.build(output, bindgen, COMPONENT, WRAPPER)

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
