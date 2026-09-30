import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib
import unittest

from cargo import archive_bytes, build_registry, normalize_manifest, toml_bytes


class CargoDistributionTests(unittest.TestCase):
    def test_reproducible_archive_retains_exact_source_bytes(self):
        files = {"crate/src/lib.rs": b"// owned\r\n", "crate/data.bin": bytes(range(256))}
        self.assertEqual(archive_bytes(files), archive_bytes(dict(reversed(list(files.items())))))
        with tarfile.open(fileobj=io.BytesIO(archive_bytes(files))) as archive:
            self.assertEqual({member.name: archive.extractfile(member).read() for member in archive}, files)

    def test_normalization_preserves_target_features_and_registry_boundaries(self):
        workspace = dict(package={"version": "0.1.0", "edition": "2024", "rust-version": "1.92", "publish": False},
                         dependencies={"mo-common": {"path": "crates/mo-common"}, "serde": {"version": "=1.0.229", "features": ["derive"]}}, lints={"rust": {"unsafe_code": "forbid"}})
        original = dict(package={"name": "mo-embedded-sdk", "version": {"workspace": True}, "edition": {"workspace": True}, "rust-version": {"workspace": True}, "workspace": "../.."},
                        dependencies={"mo-common": {"workspace": True}},
                        target={"cfg(unix)": {"dependencies": {"serde": {"workspace": True, "features": ["std"]}}}}, lints={"workspace": True})
        normalized, deps = normalize_manifest(original, workspace, "0.1.0-dev.fixture", "sparse+https://example.invalid/index/", {"mo-common", "mo-embedded-sdk"})
        self.assertEqual(normalized["lib"]["path"], "crates/mo-embedded-sdk/src/lib.rs")
        self.assertNotIn("workspace", normalized["package"])
        self.assertEqual(deps[0]["req"], "=0.1.0-dev.fixture")
        self.assertEqual(deps[1]["target"], "cfg(unix)")
        self.assertEqual(deps[1]["features"], ["derive", "std"])
        self.assertEqual(tomllib.loads(toml_bytes(normalized).decode()), normalized)
        self.assertEqual(original["package"]["workspace"], "../..")

    def test_standard_cargo_resolves_self_contained_package_without_source_checkout(self):
        # Owned tiny fixture exercises Cargo's actual archive/index protocol. No
        # third-party downloads or receiving product builds are involved.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sdk = root / "sdk"
            crate = sdk / "crates/mo-embedded-sdk"
            (crate / "src").mkdir(parents=True)
            (crate / "src/lib.rs").write_text('pub const DATA: &[u8] = include_bytes!("../../../components/owned/data.bin");\n')
            (sdk / "components/owned").mkdir(parents=True)
            (sdk / "components/owned/data.bin").write_bytes(b"owned")
            (sdk / "Cargo.toml").write_bytes(toml_bytes({"workspace": {"package": {"version": "0.1.0", "edition": "2024", "rust-version": "1.92"}, "dependencies": {}, "lints": {}}}))
            (crate / "Cargo.toml").write_bytes(toml_bytes({"package": {"name": "mo-embedded-sdk", "version": {"workspace": True}, "edition": {"workspace": True}, "rust-version": {"workspace": True}}}))
            manifest = dict(libraries=["mo-embedded-sdk"], files=[{"path": str(p.relative_to(sdk))} for p in sdk.rglob("*") if p.is_file()])
            registry = "sparse+https://example.invalid/index/"
            build_registry(sdk, manifest, root / "registry", "0.1.0-dev.fixture", registry)
            import shutil
            shutil.rmtree(sdk)
            consumer = root / "consumer"
            (consumer / "src").mkdir(parents=True)
            (consumer / ".cargo").mkdir()
            (consumer / "Cargo.toml").write_text('[package]\nname="distribution-consumer"\nversion="0.0.0"\nedition="2024"\n[dependencies]\nmo-embedded-sdk={version="=0.1.0-dev.fixture",registry="owned"}\n')
            (consumer / "src/lib.rs").write_text('pub use mo_embedded_sdk::DATA;\n')
            (consumer / ".cargo/config.toml").write_text(f'[registries.owned]\nindex="{registry}"\n[source.owned]\nregistry="{registry}"\nreplace-with="fixture"\n[source.fixture]\nlocal-registry={json.dumps(str(root / "registry"))}\n')
            subprocess.run(["cargo", "check", "--offline", "--quiet"], cwd=consumer, check=True, capture_output=True)


if __name__ == "__main__":
    unittest.main()
