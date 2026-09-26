"""Fetch a hash-pinned official ECMA schema archive into a local ignored cache."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import urllib.request
import zipfile

URL = "https://ecma-international.org/wp-content/uploads/ECMA-376-4_5th_edition_december_2016.zip"
ARCHIVE_SHA256 = "bd25da1109f73762356596918bf5ff8b74a1331642dba5f1c1d1dfc6bed34ecd"
XSD_ARCHIVE = "OfficeOpenXML-XMLSchema-Transitional.zip"
XSD_SHA256 = "d34187520749998af306faf1b730e568b0ca6d88ad24638a407c0a9bb4ca04fc"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    archive = args.directory / "part4.zip"
    if archive.exists():
        data = archive.read_bytes()
    else:
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read(16 * 1024 * 1024 + 1)
        assert len(data) <= 16 * 1024 * 1024, "official archive exceeded declared download budget"
    assert digest(data) == ARCHIVE_SHA256, "official archive changed; review before updating the pin"
    archive.write_bytes(data)
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        schemas = package.read(XSD_ARCHIVE)
    assert digest(schemas) == XSD_SHA256
    (args.directory / XSD_ARCHIVE).write_bytes(schemas)
    output = args.directory / "xsd"
    output.mkdir(exist_ok=True)
    files = []
    with zipfile.ZipFile(io.BytesIO(schemas)) as package:
        for entry in package.infolist():
            assert entry.filename == Path(entry.filename).name and "\\" not in entry.filename
            assert entry.filename.endswith(".xsd")
            content = package.read(entry)
            (output / entry.filename).write_bytes(content)
            files.append({"name": entry.filename, "sha256": digest(content)})
    receipt = {"source": URL, "archiveSha256": ARCHIVE_SHA256, "schemaArchiveSha256": XSD_SHA256, "files": files, "scope": "Official Transitional XSD validation inputs, cached for development only; not copied into runtime or presented as complete Office conformance."}
    (args.directory / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"schemaFiles": len(files), "archiveSha256": ARCHIVE_SHA256}))


if __name__ == "__main__":
    main()
