from pathlib import Path
import unittest

from build_record import relative_paths


class BuildRecordTests(unittest.TestCase):
    def test_short_container_root_preserves_nested_source_directories(self):
        record = {"headers": [{"path": "/src/dependencies/jpeg/src/cderror.h"}],
                  "commands": [["/usr/bin/clang", "/src", "/src/dependencies/jpeg/src/a.c",
                                "-I/src/include"]], "byteLength": 42}
        self.assertEqual(relative_paths(record, Path('/src')), {
            "headers": [{"path": "dependencies/jpeg/src/cderror.h"}],
            "commands": [["/usr/bin/clang", ".", "dependencies/jpeg/src/a.c", "-I/src/include"]],
            "byteLength": 42})
        self.assertEqual(record['headers'][0]['path'], '/src/dependencies/jpeg/src/cderror.h')


if __name__ == '__main__':
    unittest.main()
