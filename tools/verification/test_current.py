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
        self.assertIn("video-generated-transactions", names)
        self.assertIn("video-gpu-component-identity", names)
        self.assertIn("media-native-callbacks", names)
        self.assertIn("media-encoding-candidates", names)
        self.assertLess(names.index("media-native-callbacks"), names.index("media-rust-ownership"))
        self.assertLess(names.index("media-native-callbacks"), names.index("media-worker-execution"))
        self.assertLess(names.index("media-worker-bundle"), names.index("media-worker-execution"))
        self.assertLess(names.index("media-worker-execution"), names.index("video-source-projection"))
        self.assertLess(names.index("media-native-callbacks"), names.index("video-container-index"))
        self.assertLess(names.index("video-source-projection"), names.index("video-color-composition"))
        self.assertIn("--ignored", next(s for s in steps if s.name == "video-generated-transactions").argv)
        for dependency in ("video-conformance-native", "video-conformance-wasm", "video-conformance-bindgen"):
            self.assertLess(names.index(dependency), names.index("video-domain-native-wasm-parity"))
            self.assertLess(names.index(dependency), names.index("video-projection-native-wasm-parity"))
            self.assertLess(names.index(dependency), names.index("video-render-native-wasm-parity"))
            self.assertLess(names.index(dependency), names.index("video-audio-native-wasm-parity"))
            self.assertLess(names.index(dependency), names.index("video-pcm-native-wasm-parity"))
            self.assertLess(names.index(dependency), names.index("video-export-native-wasm-parity"))
            self.assertLess(names.index(dependency), names.index("video-stretch-native-wasm-parity"))
        reports = [s.argv[-1] for s in steps if s.name.startswith("video-") and s.name.endswith("native-wasm-parity")]
        self.assertEqual(len(reports), 8)
        self.assertEqual(len(set(reports)), len(reports), "Each evidence file must remain independently readable")
        self.assertIn("--ignored", next(s for s in steps if s.name == "native-render-tests").argv)

    def test_independent_video_authoring_builds_before_using_its_package(self):
        steps = current.plan({"video-authoring"}, Path("/output"), "/tool/wasm-bindgen", sys.executable)
        names = [s.name for s in steps]
        for dependency in ("video-conformance-native", "video-authoring-client", "video-authoring-sdk", "video-authoring-bindgen"):
            self.assertLess(names.index(dependency), names.index("video-operations-native-wasm-parity"))
        self.assertLess(names.index("video-authoring-sdk"), names.index("video-authoring-package-runtime"))
        self.assertNotIn("native-build", names, "Independent video authoring must not require PPT workers")

    def test_protocol_selection_declares_its_build_dependencies(self):
        self.assertEqual(current.selected_groups("mcp-protocol"), {"native", "mcp", "mcp-protocol"})
        with self.assertRaises(ValueError):
            current.selected_groups("unknown")

    def test_media_plan_consumes_explicit_component_and_missing_record_fails(self):
        steps = current.plan({"media"}, Path("/output"), "unused", sys.executable, "/media-component")
        self.assertEqual(len(steps), 11)
        self.assertIn("/media-component", steps[0].argv)
        self.assertIn("/media-component", steps[1].argv)
        self.assertIn("/output/media-native", steps[1].argv)
        self.assertIn("/output/media-worker-bundle", steps[3].argv)
        self.assertIn("/output/media-worker", steps[4].argv)
        self.assertIn("/output/media-worker", steps[5].argv)
        self.assertIn("/output/media-worker", steps[6].argv)
        self.assertEqual(steps[6].name, "video-audio-dsp")
        self.assertEqual(steps[7].name, "video-canonical-pcm")
        self.assertIn("/output/media-worker", steps[7].argv)
        self.assertEqual(steps[8].name, "video-container-index")
        self.assertIn("/output/media-native/bframes-vfr.mp4", steps[8].argv)
        self.assertIn("/output/media-worker-bundle", steps[8].argv)
        self.assertLess([s.name for s in steps].index("media-worker-bundle"), 8)
        self.assertEqual(steps[9].name, "video-indexed-source")
        self.assertIn("tools/verification/video-indexed-check.py", steps[9].argv)
        self.assertIn("--variants", steps[9].argv)
        self.assertIn("/output/media-worker-bundle", steps[9].argv)
        self.assertEqual(steps[10].name, "video-indexed-composition-export")
        self.assertIn("tools/verification/video-visual-check.py", steps[10].argv)
        self.assertIn("/output/video-indexed/timing", steps[10].argv)
        self.assertIn("/output/video-indexed/open-gop.mp4", steps[10].argv)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            result = subprocess.run([sys.executable, str(Path(current.__file__)), "--groups", "media", "--output", str(output),
                                     "--ffmpeg-dir", str(Path(directory) / "missing")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            report = json.loads((output / "report.json").read_text())
            self.assertTrue(any("native-build.json" in e for e in report["prerequisiteErrors"]))
            self.assertEqual(report["steps"][0]["status"], "not-run")

    def test_encode_gate_requires_real_component_and_generates_new_candidates(self):
        steps = current.plan({"encode"}, Path("/output"), "unused", sys.executable, "/media-component")
        self.assertEqual(len(steps), 4)
        self.assertEqual(steps[2].name, "encoding-worker-bundle")
        self.assertIn("--worker-bundle", steps[3].argv)
        self.assertEqual(steps[0].name, "media-encoding-candidates")
        self.assertIn("/media-component", steps[0].argv)
        self.assertIn("/output/media-encode", steps[0].argv)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            result = subprocess.run([sys.executable, str(Path(current.__file__)), "--groups", "encode", "--output", str(output),
                                     "--ffmpeg-dir", str(Path(directory) / "missing")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            report = json.loads((output / "report.json").read_text())
            self.assertTrue(any("native-build.json" in e for e in report["prerequisiteErrors"]))
            self.assertEqual(report["steps"][0]["status"], "not-run")

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
