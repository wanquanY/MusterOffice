"""Verify raw PCM/movie edit mapping using only pinned registry archives."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location("video_consumer", Path(__file__).with_name("check-rust.py"))
consumer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(consumer)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["bundle", "worker", "fixture", "output"]:
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--worker-sha256", required=True)
    parser.add_argument("--streamed", action="store_true", help="Verify EOF-sealed PCM tiles and completion record restore (requires the streaming SDK).")
    args = parser.parse_args()
    fixture_digest = hashlib.sha256(args.fixture.read_bytes()).hexdigest()
    result = consumer.check(
        args.bundle.resolve(), args.sha256, args.output.resolve(),
        program=Path(__file__).with_name("pcm-stream-consumer.rs" if args.streamed else "audio-movie-consumer.rs").read_text(),
        root_crate="mo-video-runtime", features=["native", "gpu"],
        environment={"MO_CONSUMER_WORKER": str(args.worker.resolve()),
                     "MO_CONSUMER_WORKER_SHA256": args.worker_sha256,
                     "MO_CONSUMER_FIXTURE": str(args.fixture.resolve()),
                     "MO_CONSUMER_FIXTURE_SHA256": fixture_digest},
    )
    result.update(workerManifestSha256=args.worker_sha256, fixtureSha256=fixture_digest,
                  reference="Fresh pinned decoder in demuxer mode; fixture memory PCM host",
                  streamedCanonicalPcm=args.streamed)
    (args.output / "report.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
