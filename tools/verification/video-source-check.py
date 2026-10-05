"""Compare actual compiled forward/reverse/scrub selection against independent decoded pixels."""
import argparse
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import subprocess
import struct

ROOT = Path(__file__).resolve().parents[2]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def time(value):
    return Fraction(int(value['ticks']), value['timescale'])

def continuous_fixture_span(data):
    # Independent oracle for THIS fixture's version-0, single-rate edit list.
    # Not a general parser or production inference from AVStream.duration.
    def boxes(start, end):
        out = []
        while start + 8 <= end:
            length, kind = struct.unpack_from('>I4s', data, start)
            assert length >= 8 and start + length <= end
            out.append((kind, start + 8, start + length))
            start += length
        assert start == end
        return out
    def one(rows, kind):
        matches = [(a, z) for t, a, z in rows if t == kind]
        assert len(matches) == 1
        return matches[0]
    a, z = one(boxes(0, len(data)), b'moov')
    movie = boxes(a, z)
    a, z = one(movie, b'mvhd')
    assert data[a] == 0
    movie_scale = struct.unpack_from('>I', data, a + 12)[0]
    tracks = [(a, z) for t, a, z in movie if t == b'trak']
    a, z = tracks[0]
    a, z = one(boxes(a, z), b'edts')
    a, z = one(boxes(a, z), b'elst')
    assert data[a] == 0 and struct.unpack_from('>I', data, a + 4)[0] == 1
    duration, media_start, rate_integer, rate_fraction = struct.unpack_from('>IiHH', data, a + 8)
    assert media_start == 2002 and (rate_integer, rate_fraction) == (1, 0)
    return Fraction(duration, movie_scale)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-evidence', type=Path, required=True)
    parser.add_argument('--media-evidence', type=Path, required=True)
    parser.add_argument('--worker-bundle', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    native = args.native_evidence.resolve()
    media = args.media_evidence.resolve()
    bundle = args.worker_bundle.resolve()
    evidence = json.loads((media / 'report.json').read_text())
    native_record = json.loads((native / 'report.json').read_text())
    assert evidence['status'] == native_record['status'] == 'passed'
    assert evidence['nativeEvidenceSha256'] == sha(native / 'report.json')
    fixture = native / 'bframes-vfr.mp4'
    assert sha(fixture) == evidence['fixtureSha256'] == native_record['fixtureSha256']
    span_end = continuous_fixture_span(fixture.read_bytes())
    assert span_end == Fraction(90090, 30000)
    assert sha(bundle / 'manifest.json') == evidence['workerManifestSha256']
    raw = media / 'reference.yuv'
    assert sha(raw) == evidence['videoSha256']
    env = dict(os.environ)
    env.update(MO_MEDIA_TEST_FIXTURE=str(fixture), MO_MEDIA_TEST_BUNDLE=str(bundle),
               MO_MEDIA_TEST_BUNDLE_SHA256=evidence['workerManifestSha256'],
               MO_VIDEO_SOURCE_TEST_OUTPUT=str(output / 'selections.json'))
    commands = []
    for name, extra in [('unit', []), ('projection', ['--test', 'native_projection', '--', '--ignored', '--nocapture'])]:
        cmd = ['cargo', 'test', '--offline', '--locked', '-p', 'mo-video-source', '--features', 'native', *extra]
        commands.append(cmd)
        result = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output / (name + '.stdout')).write_bytes(result.stdout)
        (output / (name + '.stderr')).write_bytes(result.stderr)
        result.check_returncode()
    lines = (native / 'video.framemd5').read_text().splitlines()
    base = Fraction(next(s for s in lines if s.startswith('#tb 0:')).split(':', 1)[1].strip())
    pts = [int(s.split(',')[2]) * base for s in lines if s and not s.startswith('#')]
    frame_size = 128 * 96 * 3 // 2
    data = raw.read_bytes()
    hashes = [hashlib.sha256(data[i:i+frame_size]).hexdigest() for i in range(0, len(data), frame_size)]
    assert len(pts) == len(hashes) == 77
    rows = json.loads((output / 'selections.json').read_text())
    assert {k: len(v) for k, v in rows.items()} == {'forward': 90, 'reverse': 90, 'scrub': 8}
    peak = 0
    for phase, entries in rows.items():
        for i, row in enumerate(entries):
            sample = row['sample']
            t = Fraction(int(sample['time']['numerator']), int(sample['time']['denominator']))
            before = sample['side'] == 'before'
            if phase == 'forward':
                assert not before and t == i * Fraction(1001, 30000)
            elif phase == 'reverse':
                assert before and t == (90 - i) * Fraction(1001, 30000)
            # Independent linear oracle over CLI presentation timestamps, not
            # sampler metadata, packet duration, or average-FPS frame numbers.
            eligible = [j for j, p in enumerate(pts) if p < t or (not before and p == t)]
            assert eligible
            j = eligible[-1]
            expected_end = pts[j+1] if j+1 < len(pts) else span_end
            assert row['sha256'] == hashes[j], (phase, i, 'pixel mismatch')
            assert row['pts'] * Fraction(row['timeBase']['numerator'], row['timeBase']['denominator']) == pts[j]
            assert time(row['interval']['start']) == pts[j]
            assert time(row['interval']['end']) == expected_end
            peak = max(peak, row['retainedBytes'])
    assert peak <= frame_size * 12
    assert any(r['cacheHit'] for r in rows['reverse'])
    assert any(r['seeks'] for r in rows['scrub'])
    report = {'format': 'musteroffice.video-source-check/1', 'status': 'passed', 'commands': commands,
              'fixtureSha256': sha(fixture), 'workerManifestSha256': evidence['workerManifestSha256'],
              'mediaEvidenceSha256': sha(media / 'report.json'), 'selectionsSha256': sha(output / 'selections.json'),
              'continuousSpanEnd': str(span_end), 'queries': sum(map(len, rows.values())), 'peakRetainedActiveBytes': peak,
              'scope': 'Actual compiled plans selecting worker-decoded frames in a known continuous VFR fixture. No general edit-list parser, color rendering, audio DSP, browser or product acceptance.'}
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))

if __name__ == '__main__':
    main()
