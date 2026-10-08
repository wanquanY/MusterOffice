"""Run the explicit hardware GPU gate against independently checked real pixels.

Missing GPU is failure, never a successful fallback/skip. Generic hosted CI does
not claim to satisfy this gate; run it on a machine with a real supported adapter.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import current

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--linear-evidence', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    source = args.linear_evidence.resolve()
    evidence = json.loads((source/'report.json').read_text())
    pixels = source/'rust/linear-rgb.f32'
    assert evidence['format'] == 'musteroffice.video-render-check/1'
    assert evidence['status'] == 'passed' and evidence['frames'] == 77
    assert sha(pixels) == evidence['linearRgbSha256']
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    before = current.source_identity()
    env = dict(os.environ, MO_VIDEO_GPU_LINEAR_INPUT=str(pixels), MO_VIDEO_GPU_REPORT=str(output/'hardware.json'))
    commands = [['python3','tools/components/record-video-gpu.py'],
                ['cargo','test','--offline','--locked','-p','mo-video-render'],
                ['cargo','test','--offline','--locked','-p','mo-video-gpu','--test','hardware','--','--ignored','--nocapture']]
    for index, command in enumerate(commands):
        result = subprocess.run(command, cwd=current.ROOT, env=env, capture_output=True, timeout=300)
        (output/f'{index}.stdout').write_bytes(result.stdout)
        (output/f'{index}.stderr').write_bytes(result.stderr)
        result.check_returncode()
    hardware = json.loads((output/'hardware.json').read_text())
    assert hardware['status'] == 'passed' and hardware['realFrames'] == 4
    assert len(hardware['cases']) == 29 and hardware['cancellationPoints'] > 0
    assert hardware['backend'] != 'Noop' and hardware['deviceType'] != 'Cpu'
    assert before == current.source_identity(), 'source changed while gate was running'
    report = {'format':'musteroffice.video-gpu-gate/1','status':'passed','commands':commands,'sourceSha256':before,
              'linearEvidenceSha256':sha(source/'report.json'),'linearRgbSha256':sha(pixels),
              'hardwareEvidenceSha256':sha(output/'hardware.json'),
              'componentRecordSha256':sha(current.ROOT/'components/video-gpu/component.json'),
              'shaderSha256':sha(current.ROOT/'crates/mo-video-gpu-plan/src/compose.wgsl'),
              'adapter':hardware['adapter'],'backend':hardware['backend'],'cases':len(hardware['cases']),
              'maxLinearError':max(c['maxLinearError'] for c in hardware['cases']),
              'maxRgba8Error':max(c['maxRgba8Error'] for c in hardware['cases']),
              'scope':hardware['scope']}
    (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:report[k] for k in ('status','adapter','backend','cases','maxLinearError','maxRgba8Error')}))

if __name__ == '__main__':
    main()
