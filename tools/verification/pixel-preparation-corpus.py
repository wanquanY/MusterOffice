"""Pixel preparation boundaries, pinned to an explicitly supplied old renderer.

Generated RGBA resources are original algorithmic fixtures, with independent
integer normalization checks. Whole-frame status/pixels use the frozen renderer;
this is a regression oracle, not external Office fidelity evidence.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess


def entry(path):
    b = path.read_bytes()
    return dict(path=str(path), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


def f(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def rect(left, top, right, bottom):
    out = [0, 5]
    for op, x, y in [(1, left, top), (2, right, top), (2, right, bottom), (2, left, bottom), (5, 0, 0)]:
        out += [op, f(x), f(y), 0, 0, 0, 0]
    return out


def resource(width, height, premul):
    # Cover every 8-bit channel/alpha pair in each 65536-pixel period.
    pattern = bytearray()
    for a in range(256):
        for c in range(256):
            rgb = [c, 255-c, (c*37) % 256]
            if premul:
                rgb = [(channel*a+127)//255 for channel in rgb]
            pattern.extend([*rgb, a])
    n = width*height*4
    return bytes((pattern*(n//len(pattern)+1))[:n])


def image_frame(width, height, alphas, sampling):
    n = len(alphas)
    frame = [0x4d4f534b, 6, width, height, 0, 1, 1, 5, 0, 0, n, n]
    frame += rect(0, 0, width, height)
    images = b''
    for alpha in alphas:
        frame += [len(images), width, height, alpha]
        images += resource(width, height, alpha)
    for i in range(n):
        frame += [i, 0, 0, sampling, f(1), 0, 0, 0, f(1), 0, 0, 0, f(width), f(height)]
    frame += [0, 0, 0, 0, 0, n]
    return frame, images


def prefix_frame(width, height):
    frame = [0x4d4f534b, 8, width, height, 0x642315d1, 2, 4, 10, 0, 0, 0, 0, 0, 2]
    frame += rect(0, 0, width, height) + rect(1.25, 2.5, width-3.75, height-1.5)
    frame += [1, 3]  # Capture before draw 1, then after the intervening blends.
    frame += [0, 0, 0, 0x7e359edc, 0, 0, 0, 0]
    frame += [1, 0, 0, 0x814982ca, 0, 0, 0, 0]
    frame += [1, 0, 0, 0, 0, 1, 0, 0]
    frame += [0, 0, 0, 0, 0, 2, 0, 1]
    return frame


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--reference-probe', type=Path, required=True)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    before = entry(a.reference_probe)
    cases = []

    def save(name, frame, images=None, expected=None):
        stem = a.output / f'{len(cases):03}'
        raw = struct.pack('<'+'I'*len(frame), *frame)
        stem.with_suffix('.frame').write_bytes(raw)
        incoming = struct.pack('<I', len(frame)) + raw
        args = [str(a.reference_probe)]
        if images is not None:
            args += ['--images']
            stem.with_suffix('.images').write_bytes(images)
            incoming += struct.pack('<I', len(images)) + images
        r = subprocess.run(args, input=incoming, capture_output=True, timeout=60, check=True)
        stem.with_suffix('.log').write_bytes(r.stderr)
        status, size = struct.unpack('<II', r.stdout[:8])
        assert status == 0 and size == frame[2]*frame[3]*4 and len(r.stdout) == size+8, name
        pixels = r.stdout[8:]
        if expected is not None:
            assert pixels == expected, (name, 'independent normalized resource differs')
        stem.with_suffix('.rgba').write_bytes(pixels)
        case = dict(name=name, frame=entry(stem.with_suffix('.frame')), status=status,
                    currentPixels=entry(stem.with_suffix('.rgba')))
        if images is not None:
            case['images'] = entry(stem.with_suffix('.images'))
        case['independentNormalization'] = expected is not None
        cases.append(case)

    for width, height in [(1, 1), (257, 257), (8192, 9)]:
        for alpha in [0, 1, 2, 63, 127, 128, 129, 254, 255]:
            for rgb in [0xffffff, 0xd1732b]:
                save(f'clear/{width}x{height}/alpha-{alpha}/rgb-{rgb:x}',
                     [0x4d4f534b, 4, width, height, alpha << 24 | rgb, 0, 0, 0, 0, 0])
    for width, height in [(256, 256), (257, 257), (512, 513), (1024, 1024)]:
        for alphas in [[0], [1], [0, 1, 0]]:
            frame, images = image_frame(width, height, alphas, 0)
            save(f'image/{width}x{height}/alphas-{alphas}', frame, images, resource(width, height, True))
    # Linear sampling remains bit-exact too, including transparent RGB values.
    frame, images = image_frame(257, 257, [0], 1)
    save('image/bilinear', frame, images, resource(257, 257, True))
    for width, height in [(256, 256), (257, 257), (512, 513), (1024, 1024)]:
        save(f'prefix/{width}x{height}', prefix_frame(width, height))
    assert before == entry(a.reference_probe)
    report = dict(format='musteroffice.pixel-preparation-corpus/1', status='passed',
                  referenceProbe=before, cases=cases,
                  scope='Original boundary fixtures; old whole-frame byte oracle plus independent integer image normalization.')
    (a.output / 'manifest.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(cases=len(cases), independentNormalization=sum(c['independentNormalization'] for c in cases))))


if __name__ == '__main__':
    main()
