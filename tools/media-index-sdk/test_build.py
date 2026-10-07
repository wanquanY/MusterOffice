import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("media_index_builder", Path(__file__).with_name("build.py"))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class DeliveryTests(unittest.TestCase):
    def test_pinned_delivery_rejects_modified_files_nested_manifests_and_links(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "sdk"
            root.mkdir()
            (root / "index.mjs").write_text("export const value=1;\n")
            manifest = {"format":builder.FORMAT,"releaseCleared":False,"files":builder.inventory(root)}
            (root / "bundle-manifest.json").write_text(json.dumps(manifest))
            pin = builder.sha(root / "bundle-manifest.json")
            builder.verify(root, pin)
            (root / "extra").mkdir()
            extra = root / "extra/bundle-manifest.json"
            extra.write_text("{}")
            with self.assertRaises(ValueError): builder.verify(root, pin)
            extra.unlink()
            (root / "index.mjs").write_text("changed")
            with self.assertRaises(ValueError): builder.verify(root, pin)
            (root / "index.mjs").unlink()
            (root / "index.mjs").symlink_to(root / "bundle-manifest.json")
            with self.assertRaises(ValueError): builder.verify(root, pin)
            with self.assertRaises(ValueError): builder.verify(root, "0" * 64)


if __name__ == "__main__":
    unittest.main()
