"""Offline XSD check of owned static PPTX outputs, not Office/WPS acceptance.

Usage: python3 pptx-static-schema-check.py ECMA_XSD_DIR OPC_XSD_DIR
       DUBLIN_CORE_XSD_DIR CASES_JSON NEW_REPORT_JSON
The caller supplies trusted official schemas. No schemas are downloaded and no
input document can select a schema. Uncovered XML namespaces fail explicitly.
"""
import hashlib
import json
from pathlib import Path
import sys
import zipfile
from lxml import etree


def identity(data):
    return dict(sha256=hashlib.sha256(data).hexdigest(), byteLength=len(data))


def main():
    ecma_dir, opc_dir, dc_dir, cases_file, output = map(Path, sys.argv[1:])
    ecma_dir, opc_dir, dc_dir = (p.resolve() for p in (ecma_dir, opc_dir, dc_dir))
    aliases = {}
    for file in ("dc.xsd", "dcterms.xsd", "dcmitype.xsd"):
        for protocol in ("http", "https"):
            for host in ("dublincore.org", "www.dublincore.org"):
                aliases[f"{protocol}://{host}/schemas/xmls/qdc/2003/04/02/{file}"] = dc_dir / file
    aliases["http://www.w3.org/2001/03/xml.xsd"] = dc_dir / "xml-200103.xsd"
    aliases["https://www.w3.org/2001/03/xml.xsd"] = dc_dir / "xml-200103.xsd"

    class LocalSchemas(etree.Resolver):
        def resolve(self, url, public_id, context):
            if url in aliases:
                return self.resolve_filename(str(aliases[url]), context)
            candidate = Path(url).resolve()
            if any(candidate.is_relative_to(root) for root in (ecma_dir, opc_dir, dc_dir)):
                return None
            raise ValueError(f"Schema dependency was not explicitly provided: {url}")

    parser = etree.XMLParser(resolve_entities=False, load_dtd=False, no_network=True)
    parser.resolvers.add(LocalSchemas())
    roots = {
        "http://schemas.openxmlformats.org/presentationml/2006/main": ecma_dir / "pml.xsd",
        "http://schemas.openxmlformats.org/drawingml/2006/main": ecma_dir / "dml-main.xsd",
        "http://schemas.openxmlformats.org/package/2006/content-types": opc_dir / "opc-contentTypes.xsd",
        "http://schemas.openxmlformats.org/package/2006/relationships": opc_dir / "opc-relationships.xsd",
        "http://schemas.openxmlformats.org/package/2006/metadata/core-properties": opc_dir / "opc-coreProperties.xsd",
    }
    schemas = {namespace: etree.XMLSchema(etree.parse(str(file), parser)) for namespace, file in roots.items()}
    records = []
    for case in json.loads(cases_file.read_text()):
        source = Path(case["source"])
        record = dict(name=case["name"], **identity(source.read_bytes()), parts=[])
        with zipfile.ZipFile(source) as package:
            assert package.testzip() is None
            for name in package.namelist():
                if not name.endswith((".xml", ".rels")):
                    continue
                data = package.read(name)
                xml = etree.fromstring(data, etree.XMLParser(resolve_entities=False, load_dtd=False, no_network=True))
                namespace = etree.QName(xml).namespace
                assert namespace in schemas, f"XML namespace outside this verification scope: {name}: {namespace}"
                schema = schemas[namespace]
                assert schema.validate(xml), (case["name"], name, list(schema.error_log))
                record["parts"].append(dict(part=name, namespace=namespace, **identity(data), xsdValid=True))
        records.append(record)
    materials = []
    for label, directory in [("ecma", ecma_dir), ("opc", opc_dir), ("dublin-core", dc_dir)]:
        for file in sorted(directory.glob("*.xsd")):
            materials.append(dict(group=label, name=file.name, **identity(file.read_bytes())))
    report = dict(profile="owned-static-pptx-offline-xsd/1", lxmlVersion=list(etree.LXML_VERSION),
                  inputs=records, suppliedSchemas=materials, networkDuringValidation=False,
                  entityResolution=False, schemaDefaultsAdded=False,
                  limitations=["Only the five declared XML root namespaces; unknown roots fail.",
                               "XSD is structural validation, not chart/workbook semantics, rendering fidelity or editing/playback certification.",
                               "Does not rewrite the input or replace the kernel OPC graph/digest checks."],
                  officeWpsProven=False)
    with output.open("x") as file:
        json.dump(report, file, indent=2)
        file.write("\n")
    print(json.dumps(dict(pptx=len(records), xmlParts=sum(len(r["parts"]) for r in records), allXsdValid=True)))


if __name__ == "__main__":
    main()
