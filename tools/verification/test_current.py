"""Tests of gate selection and failure reporting, without running a build."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("current_verification", Path(__file__).with_name("current.py"))
current = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = current
spec.loader.exec_module(current)


class CurrentVerificationTests(unittest.TestCase):
    def test_default_plan_covers_separate_workspaces_and_explicit_worker_tests(self):
        steps = current.plan(set(current.GROUPS), Path("/output"), "/tool/wasm-bindgen", sys.executable)
        names = [s.name for s in steps]
        self.assertEqual(len(names), len(set(names)))
        for name in ("rust-tests", "mcp-tests", "client-tests", "schemas", "document-native-wasm-parity", "native-host-export-tests", "mcp-cancellation", "mcp-http-tests", "mcp-http-protocol", "mcp-http-lifecycle", "mcp-all-features-clippy", "agent-package-tests", "mcp-agent-package"):
            self.assertIn(name, names)
        self.assertTrue(next(s for s in steps if s.name == "native-render-tests").worker)
        self.assertTrue(next(s for s in steps if s.name == "native-playback-tests").worker)
        self.assertIn("playback-client-tests", names)
        self.assertIn("editor-client-tests", names)
        self.assertLess(names.index("native-build"), names.index("editor-client-tests"))
        self.assertLess(names.index("wasm-bindgen"), names.index("editor-client-tests"))
        self.assertIn("playback-sdk-package-tests", names)
        self.assertIn("--ignored", next(s for s in steps if s.name == "native-render-tests").argv)

    def test_protocol_selection_declares_its_build_dependencies(self):
        self.assertEqual(current.selected_groups("mcp-protocol"), {"native", "mcp", "mcp-protocol"})
        with self.assertRaises(ValueError):
            current.selected_groups("unknown")

    def test_missing_components_never_report_a_successful_or_skipped_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            command = [sys.executable, str(Path(current.__file__)), "--groups", "native", "--output", str(output),
                       "--skia-dir", str(Path(directory) / "missing-skia"), "--harfbuzz-dir", str(Path(directory) / "missing-harfbuzz")]
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            report = json.loads((output / "report.json").read_text())
            self.assertEqual(report["status"], "blocked")
            self.assertTrue(report["prerequisiteErrors"])
            self.assertTrue(all(s["status"] == "not-run" for s in report["steps"]))
            preserved = (output / "report.json").read_bytes()
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual((output / "report.json").read_bytes(), preserved)


if __name__ == "__main__":
    unittest.main()
