import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("pcm_builder", Path(__file__).with_name("build.py"))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class ComponentIdentity(unittest.TestCase):
    def test_other_component_format_cannot_satisfy_pcm_pin(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "index.mjs").write_text("export const test = 1;\n")
            for format in (builder.FORMAT, "musteroffice.media-index-sdk/1-draft"):
                manifest = {"format": format, "releaseCleared": False, "files": builder.inventory(root)}
                (root / "bundle-manifest.json").write_text(json.dumps(manifest))
                pin = builder.sha(root / "bundle-manifest.json")
                if format == builder.FORMAT:
                    builder.verify(root, pin)
                else:
                    with self.assertRaises(ValueError):
                        builder.verify(root, pin)


if __name__ == "__main__":
    unittest.main()
