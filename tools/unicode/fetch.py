"""Fetch or verify exact Unicode data used to build the checked-in runtime table."""
import argparse,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/unicode'));p.add_argument('--manifest',type=Path,default=Path('crates/mo-unicode/data/manifest.json'));p.add_argument('--verify-only',action='store_true');a=p.parse_args()
manifest=json.loads(a.manifest.read_text());a.directory.mkdir(parents=True,exist_ok=True)
for item in manifest['inputs']:
    target=a.directory/item['name'];assert target.parent==a.directory
    if target.exists():data=target.read_bytes()
    elif a.verify_only:raise SystemExit(f'missing {target}')
    else:data=subprocess.check_output(['curl','--fail','--silent','--show-error','--connect-timeout','10','--max-time','120',item['url']])
    assert len(data)==item['byteLength'] and hashlib.sha256(data).hexdigest()==item['sha256'],item['name']
    if not target.exists():
        with target.open('xb') as f:f.write(data)
    print(item['name'],item['byteLength'],'verified')
