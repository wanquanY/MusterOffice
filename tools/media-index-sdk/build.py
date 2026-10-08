"""Offline immutable WASM component delivery; never downloads or publishes."""
import argparse
import importlib.util
import json
from pathlib import Path

WRAPPER = Path(__file__).resolve()
spec = importlib.util.spec_from_file_location("wasm_media_delivery", WRAPPER.parents[1] / "wasm-media-sdk/delivery.py")
delivery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(delivery)
COMPONENT = {'crate': 'mo-media-index-wasm', 'format': 'musteroffice.media-index-sdk/1-draft', 'profile': 'musteroffice.media-index/1-draft', 'allowed': ['mo-media-index-wasm', 'mo-media-isobmff', 'mo-media', 'mo-time', 'mo-common'], 'exports': ['MediaIndex'], 'factory': 'createMediaIndexRuntime', 'profileFunction': 'media_index_profile', 'package': '@musteroffice/media-index', 'scope': 'Bounded ISO BMFF metadata and exact movie time. Host owns byte identity and Worker termination. No codec, renderer, export or product acceptance.'}
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
