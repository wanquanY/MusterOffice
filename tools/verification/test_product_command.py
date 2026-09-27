import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest

from product_command import run


class CommandEvidenceTests(unittest.TestCase):
    def test_command_failure_is_preserved_and_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory() as directory, contextlib.redirect_stdout(io.StringIO()):
            root = Path(directory)
            (root / 'source.rs').write_text('original')
            self.assertEqual(run(root, root / 'logs', 'failure', ['source.rs'],
                                 [sys.executable, '-c', 'raise SystemExit(3)']), 3)
            report = root / 'logs/failure.json'
            original = report.read_bytes()
            self.assertEqual(json.loads(original)['exitCode'], 3)
            with self.assertRaises(FileExistsError):
                run(root, root / 'logs', 'failure', ['source.rs'],
                    [sys.executable, '-c', 'pass'])
            self.assertEqual(report.read_bytes(), original)

    def test_success_with_source_mutation_is_not_a_pass(self):
        with tempfile.TemporaryDirectory() as directory, contextlib.redirect_stdout(io.StringIO()):
            root = Path(directory)
            (root / 'source.rs').write_text('original')
            code = "from pathlib import Path; Path('source.rs').write_text('changed')"
            self.assertEqual(run(root, root / 'logs', 'changed', ['source.rs'],
                                 [sys.executable, '-c', code]), 2)
            evidence = json.loads((root / 'logs/changed.json').read_text())
            self.assertEqual(evidence['exitCode'], 0)
            self.assertFalse(evidence['sourceUnchanged'])

    def test_missing_inventory_cannot_create_passing_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(ValueError):
                run(root, root / 'logs', 'missing', ['source.rs'],
                    [sys.executable, '-c', 'pass'])
            self.assertFalse((root / 'logs/missing.json').exists())


if __name__ == '__main__':
    unittest.main()
