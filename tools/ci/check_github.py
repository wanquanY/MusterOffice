#!/usr/bin/env python3
"""Read-only drift audit of GitHub settings against the reviewed source policy.

Run with an administrator's authenticated gh CLI. This tool never changes GitHub.
"""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def matches(expected, actual):
    if isinstance(expected, dict):
        return isinstance(actual, dict) and all(key in actual and matches(value, actual[key])
                                               for key, value in expected.items())
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(expected) != len(actual):
            return False
        remaining = list(actual)
        for value in expected:
            index = next((i for i, item in enumerate(remaining) if matches(value, item)), None)
            if index is None:
                return False
            remaining.pop(index)
        return True
    return type(expected) is type(actual) and expected == actual


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository', default='wanquanY/MusterOffice')
    args = parser.parse_args()
    base = f'repos/{args.repository}'

    def get(path):
        output = subprocess.check_output(['gh', 'api', base + path], text=True)
        return json.loads(output) if output.strip() else None

    def check(label, expected, actual):
        if not matches(expected, actual):
            raise RuntimeError(f'GitHub policy drift: {label}')
        print(f'OK: {label}')

    policy = json.loads((ROOT / '.github/repository-settings.json').read_text())
    check('repository settings', policy['repository'], get(''))
    check('Actions permissions', policy['actions'], get('/actions/permissions'))
    check('allowed actions', policy['allowedActions'], get('/actions/permissions/selected-actions'))
    check('workflow token', policy['workflowPermissions'], get('/actions/permissions/workflow'))
    check('external workflow approval', {'approval_policy': policy['forkWorkflowApproval']},
          get('/actions/permissions/fork-pr-contributor-approval'))
    get('/vulnerability-alerts')  # 204 when enabled, nonzero gh exit otherwise.
    check('security updates', {'enabled': True, 'paused': False}, get('/automated-security-fixes'))
    check('private vulnerability reporting', {'enabled': True}, get('/private-vulnerability-reporting'))
    actual_rules = get('/rulesets')
    expected_rules = list((ROOT / '.github/rulesets').glob('*.json'))
    if len(actual_rules) != len(expected_rules):
        raise RuntimeError('Unexpected or missing repository rulesets')
    for path in expected_rules:
        expected = json.loads(path.read_text())
        entry = next((item for item in actual_rules if item['name'] == expected['name']), None)
        if entry is None:
            raise RuntimeError(f'Missing ruleset: {expected["name"]}')
        check(expected['name'], expected, get(f'/rulesets/{entry["id"]}'))


if __name__ == '__main__':
    main()
