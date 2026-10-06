"""Run actual worker AAC -> native block DSP, compared with an independent CLI decoder/filter."""
import argparse
from array import array
import hashlib
import json
import math
import os
from pathlib import Path
import platform
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
def metrics(actual, expected, trim=0):
    assert len(actual) == len(expected) and len(actual) > 2*trim
    maximum = squared = 0.0
    for i in range(trim, len(actual)-trim):
        a, b = actual[i], expected[i]
        assert math.isfinite(a) and math.isfinite(b)
        error = abs(a-b)
        maximum = max(maximum, error)
        squared += error*error
    return {'samples': len(actual)-2*trim, 'maximum': maximum, 'rms': math.sqrt(squared/(len(actual)-2*trim))}
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-evidence', type=Path, required=True)
    parser.add_argument('--media-evidence', type=Path, required=True)
    parser.add_argument('--worker-bundle', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    native, media, bundle = [p.resolve() for p in (args.native_evidence, args.media_evidence, args.worker_bundle)]
    n = json.loads((native/'report.json').read_text())
    m = json.loads((media/'report.json').read_text())
    assert n['status'] == m['status'] == 'passed'
    assert m['nativeEvidenceSha256'] == sha(native/'report.json')
    fixture = native/'bframes-vfr.mp4'
    assert sha(fixture) == n['fixtureSha256'] == m['fixtureSha256']
    assert sha(bundle/'manifest.json') == m['workerManifestSha256']
    env = dict(os.environ, MO_AUDIO_FIXTURE=str(fixture), MO_AUDIO_OUTPUT=str(output/'rust'),
               MO_MEDIA_TEST_BUNDLE=str(bundle), MO_MEDIA_TEST_BUNDLE_SHA256=m['workerManifestSha256'])
    commands = []
    def run(cmd, name):
        commands.append(cmd)
        result = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output/(name+'.stdout')).write_bytes(result.stdout)
        (output/(name+'.stderr')).write_bytes(result.stderr)
        result.check_returncode()
    run(['cargo','test','--locked','--offline','-p','mo-video-audio','-p','mo-video-compile'], 'unit')
    run(['cargo','test','--locked','--offline','-p','mo-video-audio','--test','native_audio','--','--ignored','--nocapture'], 'native')
    tool = n['commands'][0][0]
    run([tool,'-version'], 'reference-version')
    # The known source has exactly 144000 samples. Trim ONLY the older CLI's AAC
    # tail padding. Our decoder and mixer must produce their full counts unaided.
    for name, filters in [('forward',''), ('rate-44100',',aresample=44100:filter_size=128:phase_shift=10:cutoff=0.94'),
                          ('slow',',asetrate=24000,aresample=48000:filter_size=128:phase_shift=10:cutoff=0.94')]:
        run([tool,'-hide_banner','-loglevel','error','-i',str(fixture),'-map','0:a:0','-af',
             'atrim=end_sample=144000'+filters,'-c:a','pcm_f32le','-f','f32le',str(output/(name+'-reference.f32'))], name+'-reference')
    decoded = floats(output/'rust/normalized.f32')
    reference = floats(output/'forward-reference.f32')
    assert len(decoded) == len(reference) == 144000
    actual_forward = floats(output/'rust/forward.f32')
    assert actual_forward == decoded, 'unity pipeline altered actual decoded PCM'
    checks = {'decode': metrics(decoded, reference), 'reverse': metrics(floats(output/'rust/reverse.f32'), reference[::-1])}
    gain = math.pow(10., -0.3)
    envelope = [v*gain*min(1., i/24000., (144000-i)/48000.) for i,v in enumerate(reference)]
    checks['gain-fade'] = metrics(floats(output/'rust/gain-fade.f32'), envelope)
    for key in ['decode','reverse','gain-fade']:
        assert checks[key]['maximum'] < 1e-5 and checks[key]['rms'] < 1e-6, checks[key]
    for name, count in [('rate-44100',132300),('slow',288000)]:
        actual = floats(output/('rust/'+name+'.f32'))
        expected = floats(output/(name+'-reference.f32'))
        assert len(actual) == len(expected) == count
        checks[name] = {'all': metrics(actual, expected), 'interior': metrics(actual, expected, 256)}
        # Different documented kernels and boundary padding; compare passband
        # separately. Whole-file edge error remains recorded, never cropped out.
        assert checks[name]['interior']['maximum'] < 2e-5 and checks[name]['interior']['rms'] < 2e-6, checks[name]
        assert checks[name]['all']['maximum'] < 0.05, checks[name]
    rust = json.loads((output/'rust/report.json').read_text())
    report = {'format':'musteroffice.video-audio-check/1','status':'passed','platform':platform.platform(),
              'profile':rust['profile'],'commands':commands,'fixtureSha256':sha(fixture),
              'workerManifestSha256':m['workerManifestSha256'],'mediaEvidenceSha256':sha(media/'report.json'),
              'cases':rust['cases'],'errors':checks,'referenceSha256':{name:sha(output/(name+'-reference.f32')) for name in ['forward','rate-44100','slow']},
              'scope':'Actual mono AAC PCM normalization, integer reversal, gain/fade, SRC and pitch-changing half-speed; bounded mixed output blocks. No general edit-list/priming parser, preserve-pitch stretch, audio device, browser playback or product acceptance.'}
    (output/'report.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'status':report['status'],'errors':checks}))
if __name__ == '__main__':
    main()
