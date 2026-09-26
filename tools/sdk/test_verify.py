"""Integrity/path tests for the SDK reader; no SDK code is executed here."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from verify import verify


class VerifyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='mo-sdk-check-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        data = b'owned source fixture'
        (self.root / 'source.rs').write_bytes(data)
        self.manifest = dict(format='musteroffice.rust-embedded-sdk/1-draft', files=[dict(
            path='source.rs', byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())])

    def pin(self):
        raw = json.dumps(self.manifest).encode()
        (self.root / 'sdk-manifest.json').write_bytes(raw)
        return hashlib.sha256(raw).hexdigest()

    def test_actual_bytes_and_external_pin(self):
        pin = self.pin()
        self.assertEqual(verify(self.root, pin), self.manifest)
        with self.assertRaises(ValueError):
            verify(self.root, '0' * 64)
        (self.root / 'source.rs').write_bytes(b'changed')
        with self.assertRaises(ValueError):
            verify(self.root, pin)

    def test_extra_file(self):
        pin = self.pin()
        (self.root / 'extra').write_bytes(b'extra')
        with self.assertRaises(ValueError):
            verify(self.root, pin)

    def test_cross_platform_paths(self):
        for name in ['../source.rs', './source.rs', '/source.rs', 'C:\\source.rs', 'a//source.rs', '.', '']:
            with self.subTest(name=name):
                self.manifest['files'][0]['path'] = name
                with self.assertRaises(ValueError):
                    verify(self.root, self.pin())

    def test_case_aliases(self):
        self.manifest['files'].append(dict(self.manifest['files'][0], path='SOURCE.rs'))
        with self.assertRaises(ValueError):
            verify(self.root, self.pin())

    def test_file_identity_limits(self):
        for length in [True, -1, 1.5, 64 * 1024 * 1024 + 1]:
            with self.subTest(length=length):
                self.manifest['files'][0]['byteLength'] = length
                with self.assertRaises(ValueError):
                    verify(self.root, self.pin())

    def test_symlink(self):
        pin = self.pin()
        (self.root / 'source.rs').rename(self.root / 'original')
        (self.root / 'source.rs').symlink_to(self.root / 'original')
        with self.assertRaises(ValueError):
            verify(self.root, pin)


if __name__ == '__main__':
    unittest.main()
