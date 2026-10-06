"""Inventory admission failures must not load a changed development component."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("video_sdk_build", Path(__file__).with_name("build.py"))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class VideoSdkInventoryTests(unittest.TestCase):
    def test_pin_inventory_content_and_symlinks_are_all_enforced(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            content = root / "runtime.wasm"
            content.write_bytes(b"wasm-fixture")
            manifest = root / "bundle-manifest.json"
            manifest.write_text(json.dumps({"format": builder.FORMAT, "releaseCleared": False, "files": builder.inventory(root)}))
            pin = builder.sha(manifest)
            self.assertEqual(builder.verify(root, pin)["format"], builder.FORMAT)
            with self.assertRaises(ValueError):
                builder.verify(root, "0" * 64)
            content.write_bytes(b"wasm-changed")
            with self.assertRaises(ValueError):
                builder.verify(root, pin)
            content.write_bytes(b"wasm-fixture")
            extra = root / "extra.js"
            extra.write_text("unexpected")
            with self.assertRaises(ValueError):
                builder.verify(root, pin)
            extra.unlink()
            extra.symlink_to(content)
            with self.assertRaises(ValueError):
                builder.verify(root, pin)
            extra.unlink()
            manifest.write_text(manifest.read_text() + "\n")
            with self.assertRaises(ValueError):
                builder.verify(root, pin)


if __name__ == "__main__":
    unittest.main()
