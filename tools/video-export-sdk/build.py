"""Offline immutable export-planning WASM delivery; never downloads or publishes."""
import argparse
import importlib.util
import json
from pathlib import Path

WRAPPER = Path(__file__).resolve()
spec = importlib.util.spec_from_file_location("wasm_media_delivery", WRAPPER.parents[1] / "wasm-media-sdk/delivery.py")
delivery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(delivery)
COMPONENT = {
    "crate": "mo-video-export-wasm",
    "format": "musteroffice.video-export-sdk/1-draft",
    "profile": "musteroffice.project-export-worker/1-draft",
    "allowed": ["mo-video-export-wasm", "mo-video-export", "mo-video-render", "mo-video-audio",
                "mo-video-compile", "mo-video-model", "mo-media", "mo-media-isobmff", "mo-media-avc", "mo-color", "mo-time", "mo-common"],
    "exports": ["ProjectExport"],
    "factory": "createExportRuntime",
    "profileFunction": "export_profile",
    "package": "@musteroffice/video-export",
    "scope": "Shared immutable Rust export plan and finite interleaved frame/sample schedule. Exact media clocks and residual-bearing WebCodecs transport intervals; bounded exact acknowledgements and shared container/sample-clock inspection. Host owns codecs, mux, authorization, sources, candidate storage, decoded spot checks, durable jobs and Worker lifetime. No browser encoding or product acceptance; no cleared public release.",
}

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--bindgen")
    parser.add_argument("--verify", type=Path)
    parser.add_argument("--sha256")
    args = parser.parse_args()
    if args.verify:
        print(json.dumps({"status": "verified", "files": len(delivery.verify(args.verify, args.sha256, COMPONENT["format"])["files"])}))
    elif args.output and args.bindgen:
        print(json.dumps(delivery.build(args.output, args.bindgen, COMPONENT, WRAPPER)))
    else:
        parser.error("--output/--bindgen or --verify/--sha256 required")
