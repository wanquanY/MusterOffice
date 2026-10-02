"""Receipt substitution must never qualify a different native binary or SDK."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('native_image', Path(__file__).with_name('native-image.py'))
image = importlib.util.module_from_spec(spec)
spec.loader.exec_module(image)


class NativeReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'delivery').mkdir()
        (self.root / 'delivery/inspection.json').write_text('{"pages":2}\n')
        self.worker = {'sha256': 'b' * 64, 'byteLength': 100}
        self.manifest = {'version': '0.1.0-test', 'sdk': {'packages': [{}, {}]}}
        self.report = {
            'format': 'musteroffice.release-export-check/1', 'status': 'passed',
            'target': 'linux-x64', 'version': '0.1.0-test',
            'releaseManifestSha256': 'a' * 64, 'workerSha256': 'b' * 64,
            'registryPackages': 2,
            'inspectionSha256': image.digest(self.root / 'delivery/inspection.json'),
        }

    def check(self):
        (self.root / 'report.json').write_text(json.dumps(self.report))
        return image.verify_export(self.root, 'a' * 64, self.manifest, self.worker)

    def test_accepts_exact_native_registry_execution_receipt(self):
        self.assertIn('delivery/inspection.json', self.check())

    def test_rejects_substituted_release_platform_binary_or_package_set(self):
        for key, replacement in [('releaseManifestSha256', 'c' * 64), ('target', 'darwin-arm64'),
                                 ('workerSha256', 'c' * 64), ('registryPackages', 1),
                                 ('status', 'failed')]:
            with self.subTest(key=key):
                old = self.report[key]
                self.report[key] = replacement
                with self.assertRaisesRegex(ValueError, 'does not bind'):
                    self.check()
                self.report[key] = old

    def test_rejects_changed_stored_delivery_inspection(self):
        (self.root / 'delivery/inspection.json').write_text('{"pages":0}\n')
        with self.assertRaisesRegex(ValueError, 'does not bind'):
            self.check()


if __name__ == '__main__':
    unittest.main()
