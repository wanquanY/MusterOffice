"""Execute worker -> color normalization -> compiled CPU composition, with an independent color reference."""
import argparse
from array import array
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def floats(path):
    values = array('f', path.read_bytes())
    if sys.byteorder != 'little':
        values.byteswap()
    return values

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
    n = json.loads((native / 'report.json').read_text())
    m = json.loads((media / 'report.json').read_text())
    assert n['status'] == m['status'] == 'passed'
    assert m['nativeEvidenceSha256'] == sha(native / 'report.json')
    fixture = native / 'bframes-vfr.mp4'
    assert sha(fixture) == n['fixtureSha256'] == m['fixtureSha256']
    assert sha(bundle / 'manifest.json') == m['workerManifestSha256']
    env = dict(os.environ)
    env.update(MO_MEDIA_TEST_FIXTURE=str(fixture), MO_MEDIA_TEST_BUNDLE=str(bundle),
               MO_MEDIA_TEST_BUNDLE_SHA256=m['workerManifestSha256'],
               MO_VIDEO_RENDER_TEST_OUTPUT=str(output / 'rust'))
    commands = []
    def run(cmd, name):
        commands.append(cmd)
        result = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output / (name + '.stdout')).write_bytes(result.stdout)
        (output / (name + '.stderr')).write_bytes(result.stderr)
        result.check_returncode()
    run(['cargo','test','--offline','--locked','-p','mo-color','-p','mo-video-render'], 'unit')
    run(['cargo','test','--offline','--locked','-p','mo-video-render','--test','native_render','--','--ignored','--nocapture'], 'native')
    tool = n['commands'][0][0]
    run([tool,'-version'], 'reference-version')
    filter_graph = ('zscale=rangein=limited:matrixin=709:transferin=709:primariesin=709:'
                    'chromalin=left:range=full:matrix=gbr:transfer=linear:primaries=2020:'
                    'filter=bilinear:agamma=0,format=gbrpf32le')
    reference = output / 'reference.gbrf32'
    run([tool,'-hide_banner','-loglevel','error','-i',str(fixture),'-map','0:v:0',
         '-fps_mode','passthrough','-vf',filter_graph,'-f','rawvideo',str(reference)], 'reference')
    assert sha(output / 'rust/source.yuv') == m['videoSha256'], 'source pixels changed before color comparison'
    actual = floats(output / 'rust/linear-rgb.f32')
    expected = floats(reference)
    pixels = 128 * 96
    assert len(actual) == len(expected) == 77 * pixels * 3
    maximum = 0.0
    squared = 0.0
    for frame in range(77):
        for pixel in range(pixels):
            for channel, plane in enumerate([2, 0, 1]):
                a = actual[(frame*pixels + pixel)*3 + channel]
                b = expected[frame*pixels*3 + plane*pixels + pixel]
                assert math.isfinite(a) and math.isfinite(b)
                error = abs(a-b)
                maximum = max(maximum, error)
                squared += error*error
    rms = math.sqrt(squared/len(actual))
    # Linear-light FP32 tolerance. Approximate gamma is disabled in the reference;
    # this is far below one RGBA8 code step, and not a lossy-codec PSNR criterion.
    assert maximum <= 2e-6 and rms <= 2e-7, (maximum, rms)
    frames = json.loads((output / 'rust/frames.json').read_text())
    assert frames['normalizedFrames'] == 77
    assert [r['index'] for r in frames['renders']] == [0,17,40,89]
    artifacts = []
    for filename in [r['file'] for r in frames['renders']] + ['composed.png']:
        path = output / 'rust' / filename
        assert path.read_bytes().startswith(b'\x89PNG\r\n\x1a\n')
        artifacts.append({'path': 'rust/'+filename, 'sha256': sha(path), 'byteLength': path.stat().st_size})
    report = {'format':'musteroffice.video-render-check/1','status':'passed','commands':commands,
              'fixtureSha256':sha(fixture),'workerManifestSha256':m['workerManifestSha256'],
              'mediaEvidenceSha256':sha(media/'report.json'),'referenceSha256':sha(reference),
              'linearRgbSha256':sha(output/'rust/linear-rgb.f32'),'frames':77,'samples':len(actual),
              'maxAbsoluteLinearError':maximum,'rmsLinearError':rms,'artifacts':artifacts,
              'scope':'Actual SDR BT.709 YUV -> BT.1886 display-linear BT.2020 normalization and CPU plan composition. No GPU, browser media, text, advanced effects, production export or product acceptance.'}
    (output / 'report.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k:report[k] for k in ['status','frames','samples','maxAbsoluteLinearError','rmsLinearError']}))

if __name__ == '__main__':
    main()
