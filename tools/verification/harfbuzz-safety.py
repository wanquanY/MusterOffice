"""Native ASan/UBSan comparison and pinned upstream false-success counterexamples."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/harfbuzz'));a=p.parse_args();root=a.directory
cases=json.loads((root/'cases/manifest.json').read_text())['cases']
env=os.environ.copy();env['UBSAN_OPTIONS']='halt_on_error=1:print_stacktrace=1';env['ASAN_OPTIONS']='halt_on_error=1'
def call(binary,args,sanitized=False):
    result=subprocess.run([str(binary),*args],capture_output=True,check=True,env=env if sanitized else None,text=True)
    assert not result.stderr,result.stderr
    return json.loads(result.stdout)
results=[]
for case in cases:
    args=[case['font'],case['request'],case['language']]
    regular=call(root/'mo-hb-probe',args)
    actual=call(root/'asan/mo-hb-probe',args,True)
    assert actual['status']==regular['status'] and actual['words']==regular['words'],case['name']
    results.append({'name':case['name'],'status':actual['status']})
case=cases[0];args=[case['font'],case['request'],case['language']];faults=[]
for n in range(300):
    actual=call(root/'asan/mo-hb-probe',args+[str(n)],True)
    assert actual['status'] in [0,2]
    if actual['status']==2:
        assert not actual['words'] and actual['recoveryStatus']==6 and not actual['recoveryWords']
    else:
        assert actual['words']==actual['recoveryWords']
    faults.append({'failAfter':n,'status':actual['status']})
report={'format':'musteroffice.harfbuzz-sanitizers/1','addressSanitizer':True,'undefinedBehaviorSanitizer':True,
    'binarySha256':hashlib.sha256((root/'asan/mo-hb-probe').read_bytes()).hexdigest(),'cases':results,'faults':faults}
(root/'sanitizers.json').write_text(json.dumps(report,indent=2)+'\n')
font='.codex-work/font-corpus/notosans/NotoSans[wdth,wght].ttf'
baseline=call(root/'fault-observer',[font,'100000'])['first'];observed=[]
for i in range(30):
    actual=call(root/'fault-observer',[font,str(i)]);actual['failAfter']=i
    retry=actual['retry']
    if retry['shapeReturn'] and retry['bufferSuccessful'] and not retry['allocationFailed'] and retry['glyphs']!=baseline['glyphs']:
        observed.append(actual)
assert observed,'counterexample changed: investigate before changing the recovery contract'
(root/'upstream-retry-counterexamples.json').write_text(json.dumps({'fontSha256':hashlib.sha256(Path(font).read_bytes()).hexdigest(),'baseline':baseline,'observed':observed},indent=2)+'\n')
print(json.dumps({'sanitizedCases':len(results),'faultPositions':len(faults),'falseSuccessRetries':[c['failAfter'] for c in observed]}))
