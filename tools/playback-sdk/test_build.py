"""Package identity tests; no compiler, installation or third-party runtime."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('playback_sdk_build',Path(__file__).with_name('build.py'))
build=importlib.util.module_from_spec(spec);spec.loader.exec_module(build)


class PackageTests(unittest.TestCase):
    def test_external_pin_and_complete_inventory(self):
        for fault in ['pin','bytes','missing','extra','link']:
            with self.subTest(fault=fault),tempfile.TemporaryDirectory() as tmp:
                root=Path(tmp);(root/'index.mjs').write_bytes(b'original')
                m=dict(format='musteroffice.wasm-playback-sdk/1-draft',releaseCleared=False,files=build.inventory(root))
                build.write(root/'bundle-manifest.json',m);pin=build.sha(root/'bundle-manifest.json')
                self.assertEqual(build.verify(root,pin),m)
                if fault=='pin':pin='0'*64
                if fault=='bytes':(root/'index.mjs').write_bytes(b'changed')
                if fault=='missing':(root/'index.mjs').unlink()
                if fault=='extra':(root/'extra.js').write_bytes(b'extra')
                if fault=='link':(root/'linked').symlink_to(root/'index.mjs')
                with self.assertRaises(ValueError):build.verify(root,pin)

    def test_archive_is_deterministic_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)/'sdk';root.mkdir();(root/'index.mjs').write_bytes(b'export const example = true;')
            a,b=Path(tmp)/'a.gz',Path(tmp)/'b.gz'
            build.archive(root,a);build.archive(root,b);self.assertEqual(a.read_bytes(),b.read_bytes())
            with self.assertRaises(FileExistsError):build.archive(root,a)

    def test_copy_rejects_symlink_and_existing_target(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);source=root/'source';source.write_bytes(b'data');target=root/'target'
            build.copy(source,target);self.assertEqual(target.read_bytes(),b'data')
            with self.assertRaises(FileExistsError):build.copy(source,target)
            link=root/'link';link.symlink_to(source)
            with self.assertRaises(ValueError):build.copy(link,root/'not-written')


if __name__=='__main__':unittest.main()
