"""Export owned navigation documents with the pinned SDK and make native references.

Every output directory is new. This tests computation and the product Worker
input contract, not authenticated product publication or external applications.
"""

import argparse
import copy
import hashlib
import importlib.util
import json
import subprocess
from pathlib import Path


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def write(path, value):
    data = (
        value
        if isinstance(value, bytes)
        else json.dumps(value, separators=(",", ":")).encode()
    )
    with path.open("xb") as stream:
        stream.write(data)


def run(command):
    result = subprocess.run(
        [str(v) for v in command], capture_output=True, timeout=90, check=False
    )
    if result.returncode:
        raise RuntimeError(
            result.stderr.decode(errors="replace")
            + result.stdout.decode(errors="replace")
        )
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in [
        "output",
        "export-input",
        "consumer",
        "export-worker",
        "raster-worker",
        "cli",
    ]:
        parser.add_argument("--" + name, type=Path, required=True)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--corpus", type=Path)
    source.add_argument(
        "--native-sources",
        type=Path,
        help="Owned PPTX cases admitted through the public import API",
    )
    parser.add_argument("--case", action="append", dest="cases")
    parser.add_argument("--sample-time", action="append", type=int, dest="sample_times")
    parser.add_argument("--min-distinct-frames", type=int, default=3)
    parser.add_argument("--expected-pages", type=int, default=1)
    args = parser.parse_args()
    if args.min_distinct_frames < 2:
        parser.error("animated fixtures must contain at least two distinct frames")
    if args.expected_pages < 1:
        parser.error("expected page count must be positive")
    if args.sample_times is not None and (
        args.sample_times[0] < 0
        or any(a >= b for a, b in zip(args.sample_times, args.sample_times[1:]))
    ):
        parser.error("sample times must be nonnegative and strictly increasing")
    sample_plan = (
        [(ms, None) for ms in args.sample_times]
        if args.sample_times is not None
        else [
            (0, None),
            (100, "next"),
            (200, None),
            (300, "next"),
            (400, None),
            (500, "previous"),
            (600, None),
            (700, "next"),
            (800, None),
            (1100, None),
            (1300, None),
            (2200, None),
        ]
    )
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=False)
    inputs = {}

    def tracked(path):
        path = path.resolve()
        data = path.read_bytes()
        inputs[str(path)] = {"sha256": digest(data), "byteLength": len(data)}
        return data

    for p in [
        args.consumer,
        args.export_worker,
        args.raster_worker,
        args.cli,
        Path(__file__),
    ]:
        tracked(p)
    worker_sha = digest(tracked(args.export_worker))
    source_request = json.loads(tracked(args.export_input / "request.json"))
    source_assets = json.loads(tracked(args.export_input / "assets.json"))
    module = Path(__file__).with_name("scale-playback-fixtures.py")
    tracked(module)
    spec = importlib.util.spec_from_file_location("native_reference", module)
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    cases = []
    names = args.cases or [
        "finite",
        "infinite-concurrent",
        "infinite-exclusive",
        "nested-dependency",
        "cached-end",
    ]
    if any(Path(name).name != name or name in (".", "..") for name in names):
        parser.error("case names must be single file-name components")
    for name in names:
        folder = root / name
        folder.mkdir()
        input_dir = folder / "input"
        input_dir.mkdir()
        native_source = None
        if args.native_sources:
            native_source = tracked(args.native_sources / (name + ".pptx"))
            write(input_dir / "source.pptx", native_source)
            write(
                input_dir / "import-request.json",
                {
                    "documentId": "fixture:" + name,
                    "resourceId": "source:" + name,
                    "expectedSourceSha256": digest(native_source),
                },
            )
            imported = run(
                [
                    args.cli,
                    "pptx-import",
                    input_dir / "import-request.json",
                    input_dir / "source.pptx",
                ]
            )
            assert imported["status"] == "imported", imported
            snapshot = imported["snapshot"]
        else:
            authored = json.loads(
                tracked(args.corpus / (name + "-author-0.request.json"))
            )
            snapshot = authored["playback"]["snapshot"]
        request = copy.deepcopy(source_request)
        action = request["action"]
        action["baseRevision"] = snapshot["revision"]
        action["documentId"] = snapshot["document"]["id"]
        action["settings"]["renderer"]["implementationSha256"] = worker_sha
        action["settings"]["resources"] = []
        if native_source is not None:
            action["settings"]["resources"] = [
                {
                    "resourceId": "source:" + name,
                    "assetId": "source:" + name,
                }
            ]
        write(input_dir / "request.json", request)
        write(input_dir / "snapshot.json", snapshot)
        assets = [
            a
            for a in source_assets
            if a["info"]["id"] == action["settings"]["fontAssetId"]
        ]
        assert len(assets) == 1
        for asset in assets:
            assert Path(asset["file"]).name == asset["file"]
            write(input_dir / asset["file"], tracked(args.export_input / asset["file"]))
        if native_source is not None:
            assets.append(
                {
                    "file": "source.pptx",
                    "info": {
                        "id": "source:" + name,
                        "descriptor": {
                            "sha256": digest(native_source),
                            "byteLength": str(len(native_source)),
                            "mediaType": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                        },
                        "verification": "bytesSha256",
                    },
                }
            )
        write(input_dir / "assets.json", assets)
        exported = folder / "export"
        result = run(
            [args.consumer, input_dir, exported, args.export_worker, worker_sha]
        )
        assert result["committed"] is False and result["pages"] == args.expected_pages
        receipt, inspected = (
            read(exported / "receipt.json"),
            read(exported / "inspection.json"),
        )
        files = read(exported / "files.json")
        contents = bytearray()
        ranges = []
        for item in files:
            data = (exported / item["file"]).read_bytes()
            assert len(data) == int(item["asset"]["byteLength"])
            assert digest(data) == item["asset"]["sha256"]
            ranges.append(
                {
                    "assetId": item["asset"]["id"],
                    "byteOffset": str(len(contents)),
                    "byteLength": str(len(data)),
                }
            )
            contents.extend(data)
        expected = {
            key: inspected[key]
            for key in ["documentId", "revision", "semanticDigest", "settingsDigest"]
        }
        expected["renderer"] = action["settings"]["renderer"]
        delivery = {
            "width": 800,
            "delivery": {
                "bundle": receipt["bundle"],
                "expected": expected,
                "contents": ranges,
            },
        }
        write(folder / "request.json", delivery)
        write(folder / "contents.bin", bytes(contents))
        prepared = run(
            [
                args.cli,
                "delivery-playback",
                folder / "request.json",
                folder / "contents.bin",
            ]
        )
        assert prepared["status"] == "prepared", prepared
        write(folder / "prepared.json", prepared)
        material = prepared["inputs"]
        assert len(material["pages"]) == args.expected_pages
        write(folder / "page-count.json", args.expected_pages)

        def asset_bytes(asset_id, selected_ranges, packet):
            item = next(v for v in selected_ranges if v["assetId"] == asset_id)
            start = int(item["byteOffset"])
            return bytes(packet[start : start + int(item["byteLength"])])

        source = asset_bytes(material["source"]["id"], ranges, contents)
        fonts = (
            asset_bytes(material["fontBundle"]["id"], ranges, contents)
            if material["fontBundle"]
            else b""
        )
        binding = {
            "session": "musterwork:playback",
            "revision": material["source"]["sha256"],
            "generation": "1",
        }
        samples = []
        # Sample every delivered page. Histories reset when the product selects a page.
        plan = [
            (page_index, index, milliseconds, event)
            for page_index in range(args.expected_pages)
            for index, (milliseconds, event) in enumerate(sample_plan)
        ]
        for page_index, local_index, milliseconds, event in plan:
            if local_index == 0:
                events = []
            index = len(samples)
            page = dict(
                profile="drawingml-resource-page-q32-v1-draft",
                **material["pages"][page_index]["request"],
                fonts=material["fonts"],
            )
            at = {"ticks": str(milliseconds), "timescale": 1000}
            if event:
                events.append(
                    {
                        "at": at,
                        "generation": "1",
                        "sequence": len(events) + 1,
                        "event": {
                            "kind": "navigation",
                            "direction": event,
                            "target": None,
                        },
                    }
                )
            query = {
                "page": page,
                "sample": {
                    "binding": binding,
                    "at": at,
                    "history": {
                        "binding": binding,
                        "through": at,
                        "events": copy.deepcopy(events),
                    },
                },
            }
            response, metadata, pixels = native.native(
                args.raster_worker, query, source, fonts
            )
            write(folder / f"{index}.query.json", query)
            write(folder / f"{index}.response.json", metadata)
            write(folder / f"{index}.rgba", pixels)
            raster = response["info"]["page"]["page"]["scene"]["raster"]
            samples.append(
                {
                    "pageIndex": page_index,
                    "milliseconds": milliseconds,
                    "input": event,
                    "width": raster["width"],
                    "height": raster["height"],
                    "pixels": f"{index}.rgba",
                    "sha256": digest(pixels),
                }
            )
        assert len({s["sha256"] for s in samples}) >= args.min_distinct_frames, (
            "fixture must exhibit its declared number of distinct animation poses"
        )
        write(folder / "samples.json", samples)
        cases.append(name)
    write(root / "cases.json", cases)
    for name, entry in inputs.items():
        data = Path(name).read_bytes()
        assert len(data) == entry["byteLength"] and digest(data) == entry["sha256"], (
            name
        )
    write(
        root / "report.json",
        {
            "inputs": inputs,
            "cases": cases,
            "expectedPages": args.expected_pages,
            "nativeFrames": len(cases) * len(sample_plan) * args.expected_pages,
            "status": "passed",
            "committed": False,
            "exportWorkerSha256": worker_sha,
        },
    )
    print(
        json.dumps(
            {
                "cases": len(cases),
                "nativeFrames": len(cases) * len(sample_plan) * args.expected_pages,
                "status": "passed",
            }
        )
    )


if __name__ == "__main__":
    main()
