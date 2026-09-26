"""Exercise actual CLI page publication, refusal to overwrite and atomic failures."""
import hashlib,json,subprocess,tempfile
from pathlib import Path
ROOT=Path('.codex-work/source-page');sha=lambda b:hashlib.sha256(b).hexdigest()
parity=json.loads((ROOT/'parity.json').read_text());cases={c['name']:c for c in parity['cases'] if c['kind']=='raster'}
results=[]
with tempfile.TemporaryDirectory(prefix='cli-',dir=ROOT) as directory:
    directory=Path(directory)
    def run(name,path):
        c=cases[name]
        assert sha(Path(c['sourcePath']).read_bytes())==c['sourceSha256']
        assert sha(Path(c['requestPath']).read_bytes())==c['requestSha256']
        return subprocess.run(['target/release/mo-cli','render-pptx-page',c['requestPath'],c['sourcePath'],str(path)],capture_output=True,timeout=40)
    good=directory/'page.rgba';r=run('layers-None-None',good)
    assert r.returncode==0,r.stderr
    expected=Path(cases['layers-None-None']['responsePath']).read_bytes()
    assert r.stdout.rstrip(b'\n')==expected.rstrip(b'\n')
    assert sha(good.read_bytes())==cases['layers-None-None']['pixelsSha256']
    results.append({'name':'publish-new-page','pixelsSha256':sha(good.read_bytes()),'responseSha256':sha(expected),'byteLength':good.stat().st_size})
    before=good.read_bytes();r=run('layers-None-None',good)
    assert r.returncode!=0 and good.read_bytes()==before and not r.stdout
    assert sorted(p.name for p in directory.iterdir())==['page.rgba']
    results.append({'name':'refuse-existing-output','exitStatus':r.returncode,'preservedSha256':sha(before),'temporaryFilesLeft':0})
    for name in ['visible-text','digest-conflict','bad-viewport']:
        output=directory/(name+'.rgba');r=run(name,output)
        assert r.returncode==0 and json.loads(r.stdout)['status']=='error' and not output.exists()
        assert sorted(p.name for p in directory.iterdir())==['page.rgba']
        results.append({'name':name,'error':json.loads(r.stdout)['error']['code'],'published':False,'temporaryFilesLeft':0})
result={'format':'musteroffice.source-page-cli/1','nativeCliSha256':sha(Path('target/release/mo-cli').read_bytes()),'nativeRasterWorkerSha256':sha(Path('target/release/mo-raster-worker').read_bytes()),'parityReportSha256':sha((ROOT/'parity.json').read_bytes()),'cases':results}
(ROOT/'cli.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'checks':len(results)}))
