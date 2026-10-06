"""A successful media run must consume binaries built from the current C ABI/source."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("media_check", Path(__file__).with_name("media-native-check.py"))
media = importlib.util.module_from_spec(spec)
spec.loader.exec_module(media)


class ComponentIdentityTests(unittest.TestCase):
    def record(self):
        lock = json.loads((media.ROOT / "components/ffmpeg/lock.json").read_text())
        return {"lock": lock, "upstream": {"lock": lock, "patches": [
                    {"path": str(p.relative_to(media.ROOT / "components/ffmpeg")),
                     "byteLength": p.stat().st_size, "sha256": media.sha(p)}
                    for p in sorted((media.ROOT / "components/ffmpeg/patches").glob("*.patch"))]}, "sanitizers": False,
                "sources": [{"path": name, "sha256": hashlib.sha256((media.ROOT / "components/ffmpeg" / name).read_bytes()).hexdigest()}
                            for name in ("mo_media.c", "mo_media_decode.c", "mo_media.h", "mo_media_internal.h", "mo_encode.c", "mo_encode_mux.c", "mo_encode.h", "mo_encode_internal.h")]}

    def refused(self, record, message):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            (directory / "native-build.json").write_text(json.dumps(record))
            with patch.object(sys, "argv", ["check", "--component", str(directory), "--output", str(directory / "output")]):
                with self.assertRaisesRegex(ValueError, message):
                    media.main()

    def test_stale_c_source_is_rejected_before_any_tool_or_library_load(self):
        record = self.record()
        record["sources"][0]["sha256"] = "0" * 64
        self.refused(record, "stale")

    def test_missing_abi_header_and_wrong_build_profile_are_not_accepted(self):
        record = self.record()
        record["sources"].pop()
        self.refused(record, "inventory")
        record = self.record()
        record["sanitizers"] = True
        self.refused(record, "sanitizer profile")

    def test_missing_or_stale_vendor_patch_is_not_accepted(self):
        record = self.record()
        record["upstream"].pop("patches")
        self.refused(record, "patch inventory/digest")
        record = self.record()
        record["upstream"]["patches"][0]["sha256"] = "0" * 64
        self.refused(record, "patch inventory/digest")


if __name__ == "__main__":
    unittest.main()
