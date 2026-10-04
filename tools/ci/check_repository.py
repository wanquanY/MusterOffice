#!/usr/bin/env python3
"""Check maintained contributor entry points, example provenance and workflow pins."""
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET
import zipfile

import yaml

ROOT = Path(__file__).resolve().parents[2]
DOCS = ['README.md', 'README.zh-CN.md', 'CONTRIBUTING.md', 'CONTRIBUTING.zh-CN.md',
        'SECURITY.md', 'CODE_OF_CONDUCT.md', 'THIRD_PARTY_NOTICES.md',
        'examples/templates/README.md', 'docs/governance/licensing.md',
        'docs/governance/github-collaboration.md']


def check_links(path):
    text = path.read_text()
    for link in re.findall(r'\]\(([^)]+)\)', text):
        if '://' in link or link.startswith(('#', 'mailto:')):
            continue
        target = link.split('#')[0]
        if target and not (path.parent / target).exists():
            raise ValueError(f'{path.relative_to(ROOT)}: missing link {link}')


def check_workflow(path):
    # BaseLoader preserves the YAML key "on" instead of treating it as a boolean.
    workflow = yaml.load(path.read_text(), Loader=yaml.BaseLoader)
    events = workflow['on']
    if 'pull_request_target' in events or 'workflow_run' in events:
        raise ValueError('Untrusted PR code must never run in a privileged event')
    if workflow.get('permissions') != {'contents': 'read'}:
        raise ValueError('Workflows must default to read-only contents')
    for job in workflow['jobs'].values():
        if not job.get('timeout-minutes'):
            raise ValueError('Every job requires a bounded timeout')
        for step in job.get('steps', []):
            if 'uses' in step and not re.fullmatch(r'[\w.-]+/[\w./-]+@[0-9a-f]{40}', step['uses']):
                raise ValueError('Actions must be pinned to a full commit SHA')
            if step.get('uses', '').startswith('actions/checkout@'):
                if step.get('with', {}).get('persist-credentials') != 'false':
                    raise ValueError('Checkout must not retain credentials')


def check_templates():
    directory = ROOT / 'examples/templates'
    manifest = json.loads((directory / 'manifest.json').read_text())
    for template in manifest['templates']:
        for item in template['files']:
            path = directory / item['path']
            data = path.read_bytes()
            if len(data) != item['byteLength'] or hashlib.sha256(data).hexdigest() != item['sha256']:
                raise ValueError(f'Template manifest mismatch: {path}')
            if path.suffix == '.pptx':
                with zipfile.ZipFile(path) as archive:
                    if archive.testzip():
                        raise ValueError('PPTX CRC mismatch')
                    slides = 0
                    for name in archive.namelist():
                        if name.endswith(('.xml', '.rels')):
                            node = ET.fromstring(archive.read(name))
                            if node.tag == '{http://schemas.openxmlformats.org/presentationml/2006/main}sld':
                                slides += 1
                    if slides != template['slides']:
                        raise ValueError('Template slide count mismatch')


def main():
    for name in DOCS:
        check_links(ROOT / name)
    for path in (ROOT / '.github/workflows').glob('*.yml'):
        check_workflow(path)
    check_templates()
    if 'Apache License' not in (ROOT / 'LICENSE').read_text():
        raise ValueError('Project license is missing')
    print('Contributor links, template artifacts, license and workflow policy passed')


if __name__ == '__main__':
    main()
