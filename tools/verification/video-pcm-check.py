"""Real AAC -> canonical PCM tiles -> bounded random reads and timeline audio."""
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
from current import source_identity
ROOT = Path(__file__).resolve().parents[2]
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def floats(data):
    values = array('f', data)
    if sys.byteorder != 'little':
        values.byteswap()
    return values
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ['native-evidence','media-evidence','worker-bundle','output']:
        parser.add_argument('--'+flag, type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    native, media, bundle = [p.resolve() for p in (args.native_evidence, args.media_evidence, args.worker_bundle)]
    n = json.loads((native/'report.json').read_text())
    m = json.loads((media/'report.json').read_text())
    assert n['status'] == m['status'] == 'passed'
    assert m['nativeEvidenceSha256'] == sha(native/'report.json')
    tone = native/'bframes-vfr.mp4'
    assert sha(tone) == n['fixtureSha256'] == m['fixtureSha256']
    assert sha(bundle/'manifest.json') == m['workerManifestSha256']
    noise = output/'noise-pns.m4a'
    env = dict(os.environ, MO_PCM_TONE=str(tone), MO_PCM_NOISE=str(noise), MO_PCM_OUTPUT=str(output/'rust'),
               MO_MEDIA_TEST_BUNDLE=str(bundle), MO_MEDIA_TEST_BUNDLE_SHA256=m['workerManifestSha256'])
    source_before = source_identity()
    commands = []
    def run(cmd, name):
        commands.append(cmd)
        result = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, timeout=300)
        (output/(name+'.stdout')).write_bytes(result.stdout)
        (output/(name+'.stderr')).write_bytes(result.stderr)
        result.check_returncode()
    tool = n['commands'][0][0]
    run([tool,'-version'], 'reference-version')
    run([tool,'-hide_banner','-loglevel','error','-f','lavfi','-i',
         'anoisesrc=color=white:amplitude=0.2:sample_rate=48000:duration=3:seed=42',
         '-c:a','aac','-b:a','24k','-aac_pns','1',str(noise)], 'noise-fixture')
    run(['cargo','test','--offline','--locked','-p','mo-video-pcm'], 'unit')
    run(['cargo','test','--offline','--locked','-p','mo-video-pcm','--test','native','--','--ignored','--nocapture'], 'native')
    actual = json.loads((output/'rust/report.json').read_text())
    results = []
    for case, path in zip(actual['cases'], [tone,noise]):
        name = case['name']
        run([tool,'-hide_banner','-loglevel','error','-i',str(path),'-map','0:a:0','-af',
             'atrim=end_sample=144000','-c:a','pcm_f32le','-f','f32le',str(output/(name+'-reference.f32'))], name+'-reference')
        directory = output/'rust'/name
        continuous = (directory/'continuous.f32').read_bytes()
        assert len(continuous) == 144000*4 and sha(directory/'continuous.f32') == case['continuousSha256']
        assert (directory/'mixed-reverse.f32').read_bytes() == b''.join(continuous[i:i+4] for i in range(len(continuous)-4,-1,-4))
        queries = (directory/'queries.f32').read_bytes()
        offset = 0
        for query in case['queries']:
            first, size = query['first']*4, query['frames']*4
            window = queries[offset:offset+size]
            assert window == continuous[first:first+size]
            assert hashlib.sha256(window).hexdigest() == query['sha256']
            offset += size
        assert offset == len(queries)
        assert case['tiles'] == 36 and case['maxBuilderCarryBytes'] <= 4096*4
        assert case['maxReaderWorkingBytes'] <= 131072
        ours = floats(continuous)
        reference_path = output/(name+'-reference.f32')
        reference = floats(reference_path.read_bytes())
        assert len(reference) == len(ours)
        errors = [abs(a-b) for a,b in zip(ours,reference)]
        maximum = max(errors)
        rms = math.sqrt(sum(e*e for e in errors)/len(errors))
        assert maximum < 1e-5 and rms < 1e-6, (name,maximum,rms)
        results.append({**{k:v for k,v in case.items() if k!='queries'},'randomQueries':len(case['queries']),
                        'queryFrames':offset//4,'referenceSha256':sha(reference_path),'maximumReferenceError':maximum,'rmsReferenceError':rms})
    # This fixture demonstrates why a generic codec flush cannot serve as a
    # canonical PCM checkpoint. It does not claim that every AAC stream differs.
    assert results[1]['seekZeroAfterWarmupMaximumDifference'] > 1e-4, results[1]
    assert results[1]['afterTwentyFramesMaximumDifference'] > 1e-4, results[1]
    source_after = source_identity()
    assert source_before == source_after, 'source changed during PCM verification'
    report = {'format':'musteroffice.video-pcm-check/1','status':'passed','platform':platform.platform(),
              'profile':actual['profile'],'commands':commands,'sourceSha256':source_before,'finalSourceSha256':source_after,'workerManifestSha256':m['workerManifestSha256'],
              'mediaEvidenceSha256':sha(media/'report.json'),'cases':results,
              'scope':'Fresh continuous worker decode -> canonical local PCM tile payloads and references -> random/reverse windows and actual AudioMixer output. Bounded core payload, not RSS or full cold-seek latency. Fixture filesystem store is not the production host.'}
    (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'status':'passed','cases':results}))
if __name__ == '__main__':
    main()
