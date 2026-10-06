"""Reviewed roots and immutable profile identities, before any Cargo execution."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("video_rust", Path(__file__).with_name("rust.py"))
sdk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sdk)


class RustProfiles(unittest.TestCase):
    def test_authoring_stays_independent_and_runtime_has_explicit_native_gpu_features(self):
        authoring, _ = sdk.closure("operations")
        runtime, _ = sdk.closure("runtime")
        self.assertEqual(len(authoring), 6)
        self.assertNotIn("mo-native-media", authoring)
        self.assertNotIn("mo-video-render", authoring)
        self.assertEqual(len(runtime), 19)
        self.assertEqual(runtime["mo-video-runtime"][1]["features"]["default"], [])
        self.assertNotIn("mo-media-ffmpeg", runtime)
        self.assertNotIn("mo-stretch-sys", runtime)

    def test_manifest_cannot_change_root_duplicate_a_package_or_smuggle_an_extra_file(self):
        for selected in sdk.PROFILES.values():
            with self.subTest(root=selected["root"]), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "sample").write_bytes(b"inventory fixture, not an executable SDK")
                data = dict(format=sdk.FORMAT, releaseCleared=False, rootCrate=selected["root"],
                            profile=selected["profile"], packages=[{"name": n} for n in sorted(selected["crates"])],
                            files=sdk.wasm_builder.inventory(root))
                manifest = root / "video-rust-manifest.json"
                def save():
                    manifest.write_text(json.dumps(data))
                    return sdk.wasm_builder.sha(manifest)
                pin = save()
                sdk.verify(root, pin)
                data["rootCrate"] = "mo-unreviewed"
                with self.assertRaises(ValueError): sdk.verify(root, save())
                data["rootCrate"] = selected["root"]
                data["packages"].append(data["packages"][0])
                with self.assertRaises(ValueError): sdk.verify(root, save())
                data["packages"].pop()
                pin = save()
                (root / "unexpected").write_bytes(b"not admitted")
                with self.assertRaises(ValueError): sdk.verify(root, pin)


if __name__ == "__main__":
    unittest.main()
