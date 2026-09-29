"""Bind owned real-worker visibility observations to the public SDK replay format.

Requires successful native visibility calibration output. Adds author frames
through the public CLI, checking removal/seek against independently known poses.
"""

import argparse
import copy
import hashlib
import json
import subprocess
from pathlib import Path


def sha(data):
    return hashlib.sha256(data).hexdigest()


def record(path):
    data = path.read_bytes()
    return {"path": str(path), "byteLength": len(data), "sha256": sha(data)}


def write(path, value):
    data = json.dumps(value, indent=2).encode() + b"\n"
    with path.open("xb") as stream:
        stream.write(data)
    return record(path)


def run(cli, args, request=None):
    result = subprocess.run(
        [str(cli), *args],
        input=None if request is None else json.dumps(request).encode(),
        capture_output=True,
        timeout=60,
        check=False,
    )
    if result.returncode or result.stderr:
        raise RuntimeError(result.stderr.decode(errors="replace"))
    return json.loads(result.stdout)


def time(ms):
    return {"ticks": str(ms), "timescale": 1000}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ["native", "output", "cli", "fonts", "author"]:
        parser.add_argument("--" + key, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): record(p) for p in [args.cli, args.fonts, args.author]}
    sources = []
    for case in sorted(args.native.iterdir()):
        if not case.is_dir():
            continue
        prepare = json.loads((case / "prepare.json").read_text())
        frames = json.loads((case / "frames.json").read_text())
        for path in [case / "prepare.json", case / "frames.json", case / "input.pptx"]:
            inputs[str(path)] = record(path)
        for index, frame in enumerate(frames):
            stem = args.output / f"{case.name}-{index}"
            pixels = record(case / f"frame-{index}.rgba")
            if pixels["sha256"] != frame["sha256"]:
                raise ValueError("native pixel identity differs")
            sources.append(
                {
                    "name": stem.name,
                    "source": record(case / "input.pptx"),
                    "fonts": record(args.fonts),
                    "request": write(
                        stem.with_suffix(".request.json"),
                        {
                            "page": prepare["page"],
                            "sample": {
                                "binding": prepare["binding"],
                                "at": frame["at"],
                                "history": None,
                            },
                        },
                    ),
                    "response": write(
                        stem.with_suffix(".response.json"),
                        {"status": "rendered", "info": frame["info"]},
                    ),
                    "pixels": pixels,
                }
            )
    if len(sources) != 45:
        raise ValueError("expected five native cases with nine samples each")
    q = json.loads(args.author.read_text())
    slide = q["page"]["slide"]
    document = q["page"]["document"]
    timeline = document["timelines"][slide]
    n = copy.deepcopy(timeline["nodes"][0])
    n.update(
        start={"kind": "at", "offset": time(500)},
        duration=time(1000),
        fill="remove",
        effect={"kind": "setVisibility", "target": "group:1", "value": "hidden"},
    )
    timeline["nodes"] = [n]
    snapshot = run(args.cli, [], {"operation": "initialize", "document": document})[
        "snapshot"
    ]
    binding = {
        "session": "visibility-author",
        "revision": snapshot["revision"],
        "generation": "0",
    }
    authors = []
    poses = {}
    for index, ms in enumerate([0, 499, 500, 1000, 1499, 1500, 3000, 750, 0]):
        stem = args.output / f"author-{index}"
        request = {
            "playback": {
                "snapshot": snapshot,
                "slide": slide,
                "binding": binding,
                "at": time(ms),
                "history": None,
            },
            "viewport": q["viewport"],
            "defaults": q["defaults"],
        }
        req = write(stem.with_suffix(".request.json"), request)
        response = run(
            args.cli,
            ["render-playback-page", req["path"], str(stem.with_suffix(".rgba"))],
        )
        if response["status"] != "rendered":
            raise ValueError(response)
        pixel = record(stem.with_suffix(".rgba"))
        pose = "hidden" if 500 <= ms < 1500 else "visible"
        if poses.setdefault(pose, pixel["sha256"]) != pixel["sha256"]:
            raise ValueError("pose changed across seek/removal")
        authors.append(
            {
                "name": stem.name,
                "request": req,
                "response": write(stem.with_suffix(".response.json"), response),
                "pixels": pixel,
            }
        )
    if poses["hidden"] == poses["visible"]:
        raise ValueError("visibility did not change actual pixels")
    write(args.output / "author.json", {"cases": authors})
    write(args.output / "source.json", {"cases": sources})
    for path, expected in inputs.items():
        if record(Path(path)) != expected:
            raise ValueError("input changed")
    report = {
        "status": "passed",
        "authorFrames": len(authors),
        "sourceFrames": len(sources),
        "inputs": inputs,
        "scope": __doc__,
    }
    write(args.output / "report.json", report)
    print(
        json.dumps({k: report[k] for k in ["status", "authorFrames", "sourceFrames"]})
    )


if __name__ == "__main__":
    main()
