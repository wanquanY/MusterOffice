"""Original scan-dispatch boundary inputs; never derive expected pixels from the new renderer.

The caller supplies a frozen reference executable. These are regression and
lifetime fixtures, not evidence of Office/WPS fidelity or full PPT performance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess


def entry(path):
    raw = path.read_bytes()
    return dict(path=str(path), byteLength=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def f(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def polygon(points):
    result = [0, len(points) + 1]
    for i, (x, y) in enumerate(points):
        result += [1 if i == 0 else 2, f(x), f(y), 0, 0, 0, 0]
    return result + [5, 0, 0, 0, 0, 0, 0]


def rect(left, top, right, bottom):
    return polygon([(left, top), (right, top), (right, bottom), (left, bottom)])


def frame(width, height, shape, origin, clip):
    path = rect(0, 0, width, height) if shape == 'rect' else polygon(
        [(0, height/2), (width/2, 0), (width, height/2), (width/2, height)])
    clips = {'none': (0, 0, 64, 272), 'integer': (5, 5, 48, 270),
             'fractional': (5.25, 5.5, 48.5, 269.25), 'empty': (128, 128, 132, 132)}
    count = int(clip != 'none')
    words = [0x4d4f534b, 7, 64, 272, 0, 3, 2, 15, 0, 0, 0, 0, count]
    words += path + rect(*clips[clip]) + rect(0, 0, 2, 2)
    if count:
        words += [0, 1, 0, 0]
    words += [0, f(origin), f(origin), 0xb13bc179, 0, 0, count]
    # A completed/empty first branch must still unwind its clip and continue to
    # the following independent draw. The marker is outside all primary paths.
    words += [2, 0, 0, 0xff443322, 0, 0, 0]
    return words


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--reference-probe', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    reference = entry(args.reference_probe)
    cases = []
    # Original Skia dispatch uses width <= 32, align4(width)*height <= 1024,
    # and clipped rectangle width >= 3. Fractional placement changes rounding.
    for width, height in [(2, 8), (3, 8), (31, 32), (32, 31), (32, 32),
                          (32, 33), (33, 31), (4, 256), (4, 257)]:
        for shape in ['rect', 'diamond']:
            for origin in [4, 4.25]:
                for clip in ['none', 'integer', 'fractional', 'empty']:
                    name = f'dispatch/{width}x{height}/{shape}/{origin}/{clip}'
                    words = frame(width, height, shape, origin, clip)
                    raw = struct.pack('<' + 'I'*len(words), *words)
                    stem = args.output / f'{len(cases):03}'
                    stem.with_suffix('.frame').write_bytes(raw)
                    result = subprocess.run([str(args.reference_probe)],
                                            input=struct.pack('<I', len(words)) + raw,
                                            capture_output=True, check=True, timeout=30)
                    status, length = struct.unpack('<II', result.stdout[:8])
                    assert status == 0 and length == 64*272*4 and len(result.stdout) == length+8, name
                    pixels = result.stdout[8:]
                    assert pixels[:8] == bytes([0x22, 0x33, 0x44, 255])*2, name
                    stem.with_suffix('.rgba').write_bytes(pixels)
                    stem.with_suffix('.log').write_bytes(result.stderr)
                    cases.append(dict(name=name, frame=entry(stem.with_suffix('.frame')),
                                      status=0, currentPixels=entry(stem.with_suffix('.rgba'))))
    assert len(cases) == 144 and entry(args.reference_probe) == reference
    report = dict(format='musteroffice.scan-dispatch-corpus/1', status='passed',
                  referenceProbe=reference, cases=cases,
                  scope='Frozen whole-frame byte oracle; independent opaque trailing marker check. '
                        'Thin/fat, width/area threshold, non-rectangle, BW/AA/empty clip dispatch.')
    (args.output / 'manifest.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(cases=len(cases), status='passed')))


if __name__ == '__main__':
    main()
