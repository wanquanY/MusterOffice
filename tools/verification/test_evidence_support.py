"""Evidence must reject partial or contradictory observations."""
from pathlib import Path
import hashlib
import json
import tempfile
import unittest

from evidence_support import cargo_check, cargo_tests, local_links, recorded_command


class EvidenceSupportTests(unittest.TestCase):
    def test_command_record_rejects_changed_inputs_and_tampered_logs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / 'run.log'
            log.write_bytes(b'actual observation')
            record = dict(exitCode=0, sourceBefore={'a': 'one'}, sourceAfter={'a': 'one'},
                          logSha256=hashlib.sha256(log.read_bytes()).hexdigest())
            for broken in [dict(record, exitCode=1),
                           dict(record, sourceAfter={'a': 'two'}),
                           dict(record, logSha256='0' * 64)]:
                (root / 'run.json').write_text(json.dumps(broken))
                with self.assertRaises(ValueError):
                    recorded_command(root, 'run')
            (root / 'run.json').write_text(json.dumps(record))
            self.assertEqual(recorded_command(root, 'run'), record)

    def test_test_observations_require_complete_matching_summary(self):
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / 'tests.log'
            for text in [
                'test real ... ok\n',
                'test result: ok. 1 passed; 0 failed; 0 ignored;\n',
                'test real ... ok\ntest result: ok. 0 passed; 0 failed; 0 ignored;\n',
                'test real ... FAILED\ntest result: FAILED. 0 passed; 1 failed; 0 ignored;\n',
            ]:
                log.write_text(text)
                with self.assertRaises(ValueError):
                    cargo_tests(log)
            log.write_text('test real ... ok\ntest result: ok. 1 passed; 0 failed; 2 ignored;\n')
            self.assertEqual(cargo_tests(log), dict(passed=1, ignored=2, names=['real']))

    def test_strict_check_does_not_disguise_warning_or_missing_completion(self):
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / 'check.log'
            for text in ['Checking crate\n', 'error: fail\nFinished `dev` profile\n',
                         'warning: unresolved\nFinished `dev` profile\n']:
                log.write_text(text)
                with self.assertRaises(ValueError):
                    cargo_check(log, strict=True)

    def test_error_module_name_is_not_a_diagnostic_but_real_errors_still_fail(self):
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / 'tests.log'
            success = ('test journal::stream::error::tests::recovery ... ok\n'
                       'test result: ok. 1 passed; 0 failed; 0 ignored;\n')
            log.write_text(success)
            self.assertEqual(cargo_tests(log)['names'], ['journal::stream::error::tests::recovery'])
            for failure in ['error: test failed\n', 'error[E0308]: mismatched types\n',
                            'test additional ... FAILED\n',
                            'test result: FAILED. 0 passed; 1 failed; 0 ignored;\n']:
                log.write_text(success + failure)
                with self.assertRaises(ValueError):
                    cargo_tests(log)
            for failure in ['error: compile failed\n', 'error[E0308]: mismatched types\n']:
                log.write_text(failure + 'Finished `dev` profile\n')
                with self.assertRaises(ValueError):
                    cargo_check(log)

    def test_only_explicit_pending_links_are_exempted(self):
        with tempfile.TemporaryDirectory() as root:
            doc = Path(root) / 'record.md'
            report = Path(root) / 'evidence.json'
            doc.write_text('[record](evidence.json)')
            with self.assertRaises(ValueError):
                local_links([doc])
            self.assertEqual(local_links([doc], pending=[report]), 1)
            doc.write_text('[unrelated](absent.json)')
            with self.assertRaises(ValueError):
                local_links([doc], pending=[report])


if __name__ == '__main__':
    unittest.main()
