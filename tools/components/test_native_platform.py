import unittest
from unittest.mock import patch

from native_platform import native_link_flags, native_platform


class NativePlatformTests(unittest.TestCase):
    def test_container_identity_and_elf_linker_are_explicit(self):
        with patch("platform.system", return_value="Linux"), patch("platform.machine", return_value="x86_64"):
            target = native_platform()
        self.assertEqual(target, {"os": "linux", "arch": "x86_64"})
        flags = native_link_flags(target, "output.map")
        self.assertIn("-Wl,--gc-sections", flags)
        self.assertIn("-Wl,-Map,output.map", flags)
        self.assertNotIn("-Wl,-dead_strip", flags)

    def test_macos_profile_keeps_mach_o_linking(self):
        with patch("platform.system", return_value="Darwin"), patch("platform.machine", return_value="arm64"):
            target = native_platform()
        self.assertEqual(target, {"os": "macos", "arch": "aarch64"})
        self.assertEqual(native_link_flags(target, "out.map"), ["-Wl,-dead_strip", "-Wl,-map,out.map"])

    def test_unsupported_platform_cannot_claim_native_acceptance(self):
        with patch("platform.system", return_value="Windows"):
            with self.assertRaisesRegex(RuntimeError, "unsupported"):
                native_platform()


if __name__ == "__main__":
    unittest.main()
