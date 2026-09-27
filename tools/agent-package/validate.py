"""Validate an assembled package with explicitly supplied, pinned official schemas.

Development-only Python/jsonschema check. No network retrieval, plugin install
or global configuration writes; this does not establish client activation.
"""
import argparse
import json
from pathlib import Path

import jsonschema
from referencing import Registry

import build

SCHEMAS = {
    'plugin': '0a4aad95ce337878ad38802ebf0daa3fde76abe3f65400c86bcbb1ec0b3ab883',
    'mcp': '6539175bfcdf43085855183e86da40ea94b166547a72b47ae9a0a390516d3acb',
}


def no_retrieval(uri):
    raise ValueError('remote schema retrieval is disabled: ' + uri)


def validate(package, schemas):
    manifest = build.verify(package)
    observations = []
    for name, expected in SCHEMAS.items():
        path = schemas / (name + '.schema.json')
        if build.digest(path) != expected:
            raise ValueError('official Agent Plugins 1.0.0 schema hash differs: ' + name)
        schema = json.loads(path.read_text())
        jsonschema.Draft202012Validator.check_schema(schema)
        value = json.loads((package / (name + '.json')).read_text())
        jsonschema.Draft202012Validator(schema, registry=Registry(retrieve=no_retrieval)).validate(value)
        observations.append(dict(file=name + '.json', schema=schema['$id'], schemaSha256=expected))
    return dict(status='passed', schemas=observations, skillSha256=manifest['skillSha256'],
                installedClientAcceptance=False, releaseCleared=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('schemas', type=Path, help='directory containing the two pinned official schemas')
    args = parser.parse_args()
    print(json.dumps(validate(args.package, args.schemas), indent=2))


if __name__ == '__main__':
    main()
