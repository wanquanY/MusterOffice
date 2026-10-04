"""Reject corrupted build inputs and unsafe workflow changes before execution."""
import hashlib
from pathlib import Path
import tempfile
import unittest

from prepare_native import verify
from check_repository import check_workflow


class ProvisioningTests(unittest.TestCase):
    def test_corrupt_or_truncated_cached_source_is_rejected(self):
        expected = b'pinned upstream source'
        record = {'sha256': hashlib.sha256(expected).hexdigest(), 'byteLength': len(expected)}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'source.tar.gz'
            for invalid in [expected[:-1], b'x' * len(expected)]:
                path.write_bytes(invalid)
                with self.assertRaisesRegex(ValueError, 'Pinned source mismatch'):
                    verify(path, record)
            path.write_bytes(expected)
            verify(path, record)

    def test_privileged_event_or_unpinned_action_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'unsafe.yml'
            path.write_text('on:\n  pull_request_target:\npermissions:\n  contents: read\njobs: {}\n')
            with self.assertRaisesRegex(ValueError, 'privileged event'):
                check_workflow(path)
            path.write_text('on:\n  pull_request:\npermissions:\n  contents: read\njobs:\n'
                            '  check:\n    timeout-minutes: 5\n    steps:\n'
                            '      - uses: actions/checkout@main\n')
            with self.assertRaisesRegex(ValueError, 'full commit SHA'):
                check_workflow(path)


if __name__ == '__main__':
    unittest.main()
