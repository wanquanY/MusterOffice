"""Execute the public media SDK from registry archives and an explicitly pinned worker."""
import argparse
import importlib.util
import json
import hashlib
import subprocess
from pathlib import Path

spec = importlib.util.spec_from_file_location("video_consumer", Path(__file__).with_name("check-rust.py"))
consumer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(consumer)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--worker", type=Path, required=True)
    parser.add_argument("--worker-sha256", required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--ffmpeg", type=Path, required=True, help="independent decoder executable")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    program = Path(__file__).with_name("runtime-consumer.rs").read_text()
    output = args.output.resolve()
    result = consumer.check(args.bundle.resolve(), args.sha256, output,
        program=program, root_crate="mo-video-runtime", features=["native", "gpu"],
        environment={"MO_CONSUMER_WORKER": str(args.worker.resolve()), "MO_CONSUMER_WORKER_SHA256": args.worker_sha256,
                     "MO_CONSUMER_FIXTURE": str(args.fixture.resolve()),
                     "MO_CONSUMER_FIXTURE_SHA256": hashlib.sha256(args.fixture.read_bytes()).hexdigest(),
                     "MO_CONSUMER_OUTPUT": str(output / "media")})
    subprocess.run([str(args.ffmpeg.resolve()), "-v", "error", "-i", str(output / "media/project.mp4"),
                    "-map", "0:v:0", "-pix_fmt", "yuv420p", "-f", "rawvideo", str(output / "media/independent.yuv")], check=True)
    expected = (output / "media/expected.yuv").read_bytes()
    actual = (output / "media/independent.yuv").read_bytes()
    if expected != actual or len(actual) != 75 * 128 * 96 * 3 // 2:
        raise ValueError("independent decoded output differs from submitted project pixels")
    result.update(workerManifestSha256=args.worker_sha256,
                  fixtureSha256=hashlib.sha256(args.fixture.read_bytes()).hexdigest(),
                  independentlyDecodedBytes=len(actual), outputYuvSha256=hashlib.sha256(actual).hexdigest(),
                  independentDecoder=subprocess.check_output([str(args.ffmpeg.resolve()), "-version"], text=True).splitlines()[0])
    (output / "report.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
