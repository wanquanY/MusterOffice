import hashlib
from pathlib import Path
import stat
import tempfile
import unittest

from build import copy


class ReleaseFileTests(unittest.TestCase):
    def test_worker_mode_does_not_depend_on_source_mode_or_umask(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "worker-input"
            source.write_bytes(b"owned worker fixture")
            source.chmod(0o600)
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            copy(source, root / "worker", digest, executable=True)
            copy(source, root / "asset", digest)
            self.assertEqual(stat.S_IMODE((root / "worker").stat().st_mode), 0o755)
            self.assertEqual(stat.S_IMODE((root / "asset").stat().st_mode), 0o644)
            self.assertEqual((root / "worker").read_bytes(), source.read_bytes())
            with self.assertRaises(FileExistsError):
                copy(source, root / "worker", digest, executable=True)


if __name__ == "__main__":
    unittest.main()
