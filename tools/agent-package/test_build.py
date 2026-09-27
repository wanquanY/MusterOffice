"""Packaging invariants with synthetic binary headers; no rendering claims."""
import json
from pathlib import Path
import struct
import tempfile
import unittest
import zipfile

import build


class BuildTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.source = self.root / 'source'
        self.skill = self.source / 'integrations/skills' / build.SKILL
        self.skill.mkdir(parents=True)
        (self.skill / 'SKILL.md').write_text('---\nname: musteroffice-presentations\ndescription: Fixture\n---\n')
        (self.source / 'integrations/plugin.json').write_bytes((build.ROOT / 'integrations/plugin.json').read_bytes())
        (self.source / 'integrations/package-README.md').write_text('Fixture')
        self.mcp = self.binary('mcp', 0x100000c)
        self.worker = self.binary('worker', 0x100000c)

    def tearDown(self):
        self.tmp.cleanup()

    def binary(self, name, arch):
        path = self.root / name
        path.write_bytes(b'\xcf\xfa\xed\xfe' + struct.pack('<I', arch) + bytes(24))
        path.chmod(0o755)
        return path

    def make(self, label='one'):
        out = self.root / label / build.NAME
        build.build(out, self.mcp, self.worker, self.source)
        return out

    def test_reproducible_relocatable_archive_and_single_skill(self):
        a, b = self.make('one'), self.make('with spaces/two')
        self.assertEqual(build.verify(a), build.verify(b))
        self.assertEqual((a / 'skills' / build.SKILL / 'SKILL.md').read_bytes(), (self.skill / 'SKILL.md').read_bytes())
        aa, bb = self.root / 'a.zip', self.root / 'b.zip'
        self.assertEqual(build.archive(a, aa), build.archive(b, bb))
        with zipfile.ZipFile(aa) as z:
            self.assertIn('plugin.json', z.namelist())
            self.assertEqual(z.getinfo('bin/mo-mcp').external_attr >> 16 & 0o777, 0o755)
        self.assertFalse(build.verify(a)['releaseCleared'])

    def test_existing_package_and_archive_are_preserved(self):
        out = self.make()
        before = (out / 'bundle-manifest.json').read_bytes()
        with self.assertRaises(FileExistsError):
            build.build(out, self.mcp, self.worker, self.source)
        self.assertEqual((out / 'bundle-manifest.json').read_bytes(), before)
        archive = self.root / 'existing.zip'; archive.write_bytes(b'caller bytes')
        with self.assertRaises(FileExistsError):
            build.archive(out, archive)
        self.assertEqual(archive.read_bytes(), b'caller bytes')

    def test_wrong_target_missing_skill_and_bad_binary_fail(self):
        other = self.binary('x64', 0x1000007)
        with self.assertRaisesRegex(ValueError, 'mismatch'):
            build.build(self.root / 'wrong/musteroffice', self.mcp, other, self.source)
        (self.skill / 'SKILL.md').unlink()
        with self.assertRaisesRegex(ValueError, 'skill missing'):
            self.make()
        self.assertFalse((self.root / 'one/musteroffice').exists())
        self.mcp.write_bytes(b'not a binary')
        with self.assertRaisesRegex(ValueError, 'binary format'):
            self.make()

    def test_damage_extra_files_and_execute_flags_are_detected(self):
        out = self.make()
        binary = out / 'bin/mo-mcp'; data = binary.read_bytes()
        binary.write_bytes(data + b'changed')
        with self.assertRaises(ValueError): build.verify(out)
        binary.write_bytes(data); binary.chmod(0o644)
        with self.assertRaises(ValueError): build.verify(out)
        binary.chmod(0o755); extra = out / 'extra'; extra.write_text('unexpected')
        with self.assertRaises(ValueError): build.verify(out)
        extra.unlink(); build.verify(out)

    def test_symlinks_and_self_containing_archive_are_rejected(self):
        outside = self.root / 'outside'; outside.write_text('private input')
        (self.skill / 'linked').symlink_to(outside)
        with self.assertRaises(ValueError): self.make()
        (self.skill / 'linked').unlink()
        out = self.make()
        with self.assertRaisesRegex(ValueError, 'outside'):
            build.archive(out, out / 'recursive.zip')
        self.assertFalse((out / 'recursive.zip').exists())
        manifest = out / 'bundle-manifest.json'; manifest.unlink(); manifest.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, 'non-symlink'): build.verify(out)


if __name__ == '__main__':
    unittest.main()
