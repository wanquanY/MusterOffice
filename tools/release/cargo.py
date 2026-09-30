"""Turn the verified production SDK into standard, self-contained Cargo packages.

No product source, registry credentials, network, or compiler execution. Source
paths inside each crate retain their original layout so include_bytes! references
keep their meaning; the package contains its own required data and notices.
"""
from __future__ import annotations

import copy
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile
import tomllib

CRATES_IO = "https://github.com/rust-lang/crates.io-index"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def index_path(name: str) -> str:
    if not re.fullmatch(r"mo-[a-z0-9-]{1,60}", name):
        raise ValueError("invalid owned crate name")
    return f"{name[:2]}/{name[2:4]}/{name}"


def toml_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (str, int)):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(v) for v in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{json.dumps(k)} = {toml_value(v)}" for k, v in sorted(value.items())) + " }"
    raise ValueError("unsupported TOML value")


def toml_bytes(value):
    result = "\n".join(f"{json.dumps(k)} = {toml_value(v)}" for k, v in sorted(value.items())) + "\n"
    if tomllib.loads(result) != value:
        raise ValueError("manifest normalization failed")
    return result.encode()


def archive_bytes(files: dict[str, bytes]) -> bytes:
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as zipped:
        with tarfile.open(fileobj=zipped, mode="w", format=tarfile.PAX_FORMAT) as archive:
            for name, data in sorted(files.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(data), 0o644, 0
                archive.addfile(info, io.BytesIO(data))
    return output.getvalue()


def normalize_manifest(source, workspace, version, registry, selected):
    manifest = copy.deepcopy(source)
    package = manifest["package"]
    for key, value in list(package.items()):
        if value == {"workspace": True}:
            package[key] = workspace["package"][key]
    package.pop("workspace", None)
    package.update(version=version, publish=False, autobins=False, autoexamples=False, autotests=False, autobenches=False)
    # Deliberately preserve original source-relative resource paths.
    name = package["name"]
    manifest.setdefault("lib", {}).update(path=f"crates/{name}/src/lib.rs", test=False, doctest=False, bench=False)
    if manifest.get("lints", {}).get("workspace"):
        manifest["lints"] = copy.deepcopy(workspace["lints"])
    manifest.pop("dev-dependencies", None)
    index_dependencies = []
    tables = [(None, manifest)] + list(manifest.get("target", {}).items())
    for target, table in tables:
        table.pop("dev-dependencies", None)
        if table.get("build-dependencies"):
            raise ValueError("production SDK must not run build scripts")
        for key, declaration in table.get("dependencies", {}).items():
            dep = {"version": declaration} if isinstance(declaration, str) else copy.deepcopy(declaration)
            if dep.pop("workspace", False):
                inherited = copy.deepcopy(workspace["dependencies"][key])
                inherited = {"version": inherited} if isinstance(inherited, str) else inherited
                features = inherited.get("features", []) + dep.get("features", [])
                dep = {**inherited, **dep}
                if features:
                    dep["features"] = sorted(set(features))
            owned = dep.get("package", key) in selected
            if "path" in dep and not owned:
                raise ValueError("unknown path dependency")
            dep.pop("path", None)
            if owned:
                dep["version"] = "=" + version
                # Cargo's normalized package format uses the registry index URL.
                dep["registry-index"] = registry
            table["dependencies"][key] = dep
            index_dependencies.append(dict(
                name=key, req=dep["version"], features=dep.get("features", []),
                optional=dep.get("optional", False), default_features=dep.get("default-features", True),
                target=target, kind="normal", registry=None if owned else CRATES_IO,
                package=dep.get("package"),
            ))
    return manifest, index_dependencies


def build_registry(sdk: Path, sdk_manifest: dict, output: Path, version: str, registry: str):
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", version):
        raise ValueError("version must be immutable semver without build metadata")
    if not registry.startswith("sparse+https://") or not registry.endswith("/"):
        raise ValueError("canonical sparse HTTPS registry required")
    workspace = tomllib.loads((sdk / "Cargo.toml").read_text())["workspace"]
    selected = set(sdk_manifest["libraries"])
    records = []
    output.mkdir(parents=True, exist_ok=False)
    for name in sorted(selected):
        crate = sdk / "crates" / name
        source = tomllib.loads((crate / "Cargo.toml").read_text())
        manifest, deps = normalize_manifest(source, workspace, version, registry, selected)
        files = {"Cargo.toml": toml_bytes(manifest)}
        for entry in sdk_manifest["files"]:
            relative = entry["path"]
            own_source = relative.startswith(f"crates/{name}/") and not relative.endswith("Cargo.toml")
            # The portable SDK owns this exact production-data closure. Preserve
            # its license records as well as the external compile-time includes.
            if own_source or relative.startswith("components/"):
                files[relative] = (sdk / relative).read_bytes()
        prefix = f"{name}-{version}"
        data = archive_bytes({f"{prefix}/{key}": value for key, value in files.items()})
        filename = prefix + ".crate"
        (output / filename).write_bytes(data)
        index = dict(name=name, vers=version, deps=deps, cksum=digest(data),
                     features=manifest.get("features", {}), yanked=False, v=2,
                     rust_version=manifest["package"]["rust-version"])
        target = output / "index" / index_path(name)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(index, sort_keys=True, separators=(",", ":")) + "\n")
        records.append(dict(name=name, version=version, path=f"registry/{filename}",
                            sha256=digest(data), byteLength=len(data), index=index))
    return records
