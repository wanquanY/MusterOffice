"""Validate generated runtime contracts and optional native/WASM response corpus."""
import argparse
import copy
import json
import hashlib
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--parity-report", type=Path)
    parser.add_argument("--opc-report", type=Path)
    parser.add_argument("--source-report", type=Path)
    parser.add_argument("--source-line-report", type=Path, action="append", default=[])
    parser.add_argument("--source-fill-report", type=Path, action="append", default=[])
    parser.add_argument("--source-effect-report", type=Path, action="append", default=[])
    parser.add_argument("--source-text-report", type=Path, action="append", default=[])
    parser.add_argument("--text-body-report", type=Path, action="append", default=[])
    parser.add_argument("--native-path-report", type=Path, action="append", default=[])
    parser.add_argument("--geometry-eval-report", type=Path, action="append", default=[])
    parser.add_argument("--source-geometry-report", type=Path, action="append", default=[])
    parser.add_argument("--fill-style-report", type=Path, action="append", default=[])
    parser.add_argument("--line-style-report", type=Path, action="append", default=[])
    parser.add_argument("--fill-color-report", type=Path, action="append", default=[])
    parser.add_argument("--line-color-report", type=Path, action="append", default=[])
    parser.add_argument("--color-report", type=Path)
    parser.add_argument("--font-report", type=Path)
    parser.add_argument("--text-report", type=Path)
    parser.add_argument("--unicode-report", type=Path)
    parser.add_argument("--cascade-report", type=Path)
    parser.add_argument("--bidi-report", type=Path)
    parser.add_argument("--itemization-report", type=Path, action="append", default=[])
    parser.add_argument("--line-break-report", type=Path, action="append", default=[])
    parser.add_argument("--font-metrics-report", type=Path, action="append", default=[])
    parser.add_argument("--line-shape-report", type=Path, action="append", default=[])
    parser.add_argument("--line-geometry-report", type=Path, action="append", default=[])
    parser.add_argument("--paragraph-layout-report", type=Path, action="append", default=[])
    parser.add_argument("--font-outlines-report", type=Path, action="append", default=[])
    parser.add_argument("--paragraph-paths-report", type=Path, action="append", default=[])
    parser.add_argument("--path-raster-report", type=Path, action="append", default=[])
    parser.add_argument("--source-page-report", type=Path, action="append", default=[])
    parser.add_argument("--source-placement-report", type=Path, action="append", default=[])
    parser.add_argument("--preset-expansion-report", type=Path, action="append", default=[])
    parser.add_argument("--gradient-raster-report", type=Path, action="append", default=[])
    parser.add_argument("--scene-raster-report", type=Path, action="append", default=[])
    parser.add_argument("--page-placement-report", type=Path, action="append", default=[])
    parser.add_argument("--page-render-report", type=Path, action="append", default=[])
    args = parser.parse_args()
    schemas = {}
    for path in (ROOT / "contracts/generated").glob("*.schema.json"):
        schema = json.loads(path.read_text())
        Draft202012Validator.check_schema(schema)
        schemas[path.name] = Draft202012Validator(schema)
    assert len(schemas) == 82, "missing runtime schema"
    assert {"pptx-radial-layout-request.schema.json", "pptx-radial-layout-response.schema.json"} <= schemas.keys()
    assert {"image-decode-request.schema.json", "image-decode-response.schema.json"} <= schemas.keys()
    assert {"image-scene-request.schema.json", "image-scene-response.schema.json"} <= schemas.keys()
    assert {"pptx-image-query.schema.json", "pptx-image-response.schema.json"} <= schemas.keys()
    assert {"image-raster-request.schema.json", "image-raster-response.schema.json"} <= schemas.keys()
    assert {"pptx-resource-page-request.schema.json", "pptx-resource-page-raster-response.schema.json"} <= schemas.keys()
    assert {"pptx-text-page-request.schema.json", "pptx-text-page-raster-response.schema.json"} <= schemas.keys()
    text_body_sources = text_body_responses = text_body_requests = text_body_edits = 0
    for report_path in args.text_body_report:
        for case in json.loads(report_path.read_text())["cases"]:
            raw = (ROOT / case["responsePath"]).read_bytes()
            assert hashlib.sha256(raw).hexdigest() == case["responseSha256"]
            if case["kind"] == "source":
                schemas["pptx-source-response.schema.json"].validate(json.loads(raw))
                text_body_sources += 1
            else:
                schemas["pptx-text-body-response.schema.json"].validate(json.loads(raw))
                text_body_responses += 1
                raw = (ROOT / case["requestPath"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case["requestSha256"]
                schemas["pptx-text-body-query.schema.json"].validate(json.loads(raw))
                text_body_requests += 1
            if "editRequestPath" in case:
                raw = (ROOT / case["editRequestPath"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case["editRequestSha256"]
                schemas["pptx-text-edits.schema.json"].validate(json.loads(raw))
                text_body_edits += 1
    document = json.loads((ROOT / "fixtures/presentations/basic-shape.json").read_text())
    validator = schemas["document.schema.json"]
    validator.validate(document)
    schemas["pptx-export-request.schema.json"].validate(json.loads((ROOT / "fixtures/presentations/native-export/request.json").read_text()))
    mutations = [
        ("unknown root field", lambda d: d.update(arbitraryScript="run()")),
        ("unknown format", lambda d: d.update(format="unknown")),
        ("path as identity", lambda d: d.update(id="../document")),
        ("identity trailing newline", lambda d: d.update(id="document\n")),
        ("numeric coordinate", lambda d: d["pageSize"].update(width=12192000)),
        ("noncanonical coordinate", lambda d: d["pageSize"].update(width="-0")),
        ("coordinate trailing newline", lambda d: d["pageSize"].update(width="1\n")),
        ("fractional rotation", lambda d: d["objects"]["shape:1"]["transform"].update(rotation=1.5)),
        ("unknown content", lambda d: d["objects"]["shape:1"]["content"].update(kind="unsupported")),
    ]
    for name, mutate in mutations:
        invalid = copy.deepcopy(document)
        mutate(invalid)
        assert not validator.is_valid(invalid), f"accepted invalid contract: {name}"
    responses = 0
    requests = 1
    if args.parity_report:
        report = json.loads(args.parity_report.read_text())
        for case in report["cases"]:
            schemas["kernel-response.schema.json"].validate(case["response"])
            responses += 1
            if case["response"]["status"] != "error":
                schemas["kernel-request.schema.json"].validate(json.loads(case["input"]))
                requests += 1
    opc_responses = 0
    if args.opc_report:
        report = json.loads(args.opc_report.read_text())
        for case in report["cases"]:
            schemas["package-inspection.schema.json"].validate(case["response"])
            opc_responses += 1
    source_responses = 0
    source_requests = 0
    if args.source_report:
        report = json.loads(args.source_report.read_text())
        for case in report["cases"]:
            if "response" in case:
                schemas["pptx-source-response.schema.json"].validate(case["response"])
                source_responses += 1
        for case in report["requests"]:
            schemas["pptx-text-edits.schema.json"].validate(case["request"])
            source_requests += 1
    color_requests = 0
    def source_records(paths):
        response_count = request_count = 0
        for report_path in paths:
            for case in json.loads(report_path.read_text())["cases"]:
                response = (ROOT / case["responsePath"]).read_bytes()
                assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
                schemas["pptx-source-response.schema.json"].validate(json.loads(response))
                response_count += 1
                if "requestPath" in case:
                    request = (ROOT / case["requestPath"]).read_bytes()
                    assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
                    schemas["pptx-text-edits.schema.json"].validate(json.loads(request))
                    request_count += 1
        return response_count, request_count
    source_line_responses, source_line_requests = source_records(args.source_line_report)
    source_fill_responses, source_fill_requests = source_records(args.source_fill_report)
    source_effect_responses, source_effect_requests = source_records(args.source_effect_report)
    source_text_responses, source_text_requests = source_records(args.source_text_report)
    source_geometry_responses, source_geometry_requests = source_records(args.source_geometry_report)
    native_path_responses = native_path_requests = 0
    for report_path in args.native_path_report:
        for case in json.loads(report_path.read_text())["cases"]:
            for field, schema, enabled in [("response", "pptx-paths-response", True),
                                          ("request", "pptx-paths-query", case.get("validRequest", True)),
                                          ("evaluation", "pptx-geometry-response", "evaluationPath" in case),
                                          ("evaluationRequest", "pptx-geometry-query", "evaluationRequestPath" in case)]:
                if not enabled: continue
                raw = (ROOT / case[field + "Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case[field + "Sha256"]
                schemas[schema + ".schema.json"].validate(json.loads(raw))
                if field == "response": native_path_responses += 1
                elif field == "request": native_path_requests += 1
    geometry_eval_responses = geometry_eval_requests = geometry_eval_edits = 0
    for report_path in args.geometry_eval_report:
        for case in json.loads(report_path.read_text())["cases"]:
            for field, schema, enabled in [("response", "pptx-geometry-response", True),
                                          ("request", "pptx-geometry-query", case.get("validRequest", True)),
                                          ("editRequest", "pptx-text-edits", "editRequestPath" in case)]:
                if not enabled: continue
                raw = (ROOT / case[field + "Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case[field + "Sha256"]
                schemas[schema + ".schema.json"].validate(json.loads(raw))
                if field == "response": geometry_eval_responses += 1
                elif field == "request": geometry_eval_requests += 1
                else: geometry_eval_edits += 1
    fill_style_responses = fill_style_requests = fill_style_edits = fill_style_inspections = 0
    for report_path in args.fill_style_report:
        for case in json.loads(report_path.read_text())["cases"]:
            for field, schema, enabled in [("response", "pptx-fill-response", True),
                                          ("request", "pptx-fill-query", case.get("validRequest", True)),
                                          ("editRequest", "pptx-text-edits", "editRequestPath" in case),
                                          ("inspection", "pptx-source-response", "inspectionPath" in case)]:
                if not enabled: continue
                raw = (ROOT / case[field + "Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case[field + "Sha256"]
                schemas[schema + ".schema.json"].validate(json.loads(raw))
                if field == "response": fill_style_responses += 1
                elif field == "request": fill_style_requests += 1
                elif field == "editRequest": fill_style_edits += 1
                else: fill_style_inspections += 1
    line_style_responses = line_style_requests = line_style_edits = 0
    for report_path in args.line_style_report:
        for case in json.loads(report_path.read_text())["cases"]:
            for field, schema, enabled in [("response", "pptx-line-response", True),
                                          ("request", "pptx-line-query", case.get("validRequest", True)),
                                          ("editRequest", "pptx-text-edits", "editRequestPath" in case)]:
                if not enabled: continue
                raw = (ROOT / case[field + "Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case[field + "Sha256"]
                schemas[schema + ".schema.json"].validate(json.loads(raw))
                if field == "response": line_style_responses += 1
                elif field == "request": line_style_requests += 1
                else: line_style_edits += 1
    def color_records(paths, family):
        counts = [0, 0, 0]
        for report_path in paths:
            for case in json.loads(report_path.read_text())["cases"]:
                for i, (field, schema, enabled) in enumerate([
                    ("response", f"pptx-{family}-color-response", True),
                    ("request", f"pptx-{family}-color-query", case.get("validRequest", True)),
                    ("editRequest", "pptx-text-edits", "editRequestPath" in case),
                ]):
                    if not enabled: continue
                    raw = (ROOT / case[field + "Path"]).read_bytes()
                    assert hashlib.sha256(raw).hexdigest() == case[field + "Sha256"]
                    schemas[schema + ".schema.json"].validate(json.loads(raw))
                    counts[i] += 1
        return counts
    line_color_responses, line_color_requests, line_color_edits = color_records(args.line_color_report, "line")
    fill_color_responses, fill_color_requests, fill_color_edits = color_records(args.fill_color_report, "fill")
    color_responses = 0
    if args.color_report:
        report = json.loads(args.color_report.read_text())
        for case in report["cases"]:
            schemas["pptx-color-response.schema.json"].validate(case["response"])
            color_responses += 1
            if case.get("validRequest", True):
                schemas["pptx-color-query.schema.json"].validate(json.loads(case["request"]))
                color_requests += 1
    font_requests = 0
    font_responses = 0
    if args.font_report:
        report = json.loads(args.font_report.read_text())
        for case in report["cases"]:
            schemas["font-response.schema.json"].validate(case["response"])
            font_responses += 1
            if case.get("validRequest", True):
                schemas["font-request.schema.json"].validate(json.loads(case["request"]))
                font_requests += 1
    text_responses = 0
    unicode_responses = 0
    bidi_responses = 0
    if args.bidi_report:
        for case in json.loads(args.bidi_report.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["bidi-analysis-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["bidi-analysis-request.schema.json"].validate(json.loads(request))
            bidi_responses += 1
    cascade_responses = 0
    if args.cascade_report:
        for case in json.loads(args.cascade_report.read_text())["cases"]:
            schemas["cascade-response.schema.json"].validate(case["response"])
            if case.get("validRequest", True):
                schemas["cascade-request.schema.json"].validate(json.loads(case["request"]))
            cascade_responses += 1
    if args.unicode_report:
        for case in json.loads(args.unicode_report.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["text-analysis-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["text-analysis-request.schema.json"].validate(json.loads(request))
            unicode_responses += 1
    if args.text_report:
        for case in json.loads(args.text_report.read_text())["cases"]:
            schemas["shape-response.schema.json"].validate(case["response"])
            if case.get("validRequest", True):
                schemas["shape-request.schema.json"].validate(json.loads(case["request"]))
            text_responses += 1
    itemization_responses = 0
    paragraph_responses = 0
    line_break_responses = 0
    font_metrics_responses = 0
    font_outlines_responses = 0
    paragraph_layout_responses = 0
    paragraph_paths_responses = 0
    for report_path in args.paragraph_layout_report:
        for case in json.loads(report_path.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["paragraph-layout-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["paragraph-layout-request.schema.json"].validate(json.loads(request))
            paragraph_layout_responses += 1
    page_render_responses = 0
    for report_path in args.page_render_report:
        for case in json.loads(report_path.read_text())["cases"]:
            for key, schema in [("response", "page-raster-response"), ("plan", "page-compile-response")]:
                raw = (ROOT / case[key + "Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case[key + "Sha256"]
                schemas[schema + ".schema.json"].validate(json.loads(raw))
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            if case.get("validRequest", True):
                schemas["page-render-request.schema.json"].validate(json.loads(request))
            page_render_responses += 1
    source_page_responses = source_page_requests = 0
    for path in args.source_page_report:
        for case in json.loads(path.read_text())["cases"]:
            kind = {"source": "pptx-source-response", "compile": "pptx-page-compile-response", "raster": "pptx-page-raster-response"}[case["kind"]]
            for field in ["response", "request"]:
                if field+"Path" not in case or (field=="request" and not case.get("validRequest",True)): continue
                raw = Path(case[field+"Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest()==case[field+"Sha256"]
                schemas[(kind if field=="response" else "pptx-page-request")+".schema.json"].validate(json.loads(raw))
                if field=="response": source_page_responses += 1
                else: source_page_requests += 1
    source_placement_responses = source_placement_requests = 0
    for path in args.source_placement_report:
        for case in json.loads(path.read_text())["cases"]:
            kind = {"placement": "pptx-placement", "source": "pptx-source", "paths": "pptx-paths", "raster": "scene-raster"}[case["kind"]]
            for field in ["response", "request"]:
                if field + "Path" not in case: continue
                if field == "request" and not case.get("validRequest", True): continue
                raw = Path(case[field + "Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case[field + "Sha256"]
                suffix = "query" if field == "request" and kind in ["pptx-placement", "pptx-paths"] else field
                schemas[kind+"-"+suffix+".schema.json"].validate(json.loads(raw))
                if field == "request": source_placement_requests += 1
                else: source_placement_responses += 1
    preset_responses = preset_requests = 0
    for path in args.preset_expansion_report:
        for case in json.loads(path.read_text())["cases"]:
            kind = {"geometry": "pptx-geometry", "paths": "pptx-paths", "raster": "path-raster"}[case["kind"]]
            for field, suffix in [("request", "request" if kind == "path-raster" else "query"), ("response", "response")]:
                raw = Path(case[field + "Path"]).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == case[field + "Sha256"]
                schemas[kind+"-"+suffix+".schema.json"].validate(json.loads(raw))
                if field == "request": preset_requests += 1
                else: preset_responses += 1
    gradient_responses = 0
    gradient_requests = 0
    for path in args.gradient_raster_report:
        for case in json.loads(path.read_text())["cases"]:
            kind = "scene-raster" if case["scene"] else "path-raster"
            response = Path(case["responsePath"]).read_bytes()
            request = Path(case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas[kind+"-response.schema.json"].validate(json.loads(response))
            assert schemas[kind+"-request.schema.json"].is_valid(json.loads(request)) == case["validRequest"]
            gradient_responses += 1
            gradient_requests += case["validRequest"]
    raster_responses = {}
    for kind, report_paths in [("path-raster", args.path_raster_report), ("scene-raster", args.scene_raster_report), ("page-placement", args.page_placement_report)]:
        count = 0
        for report_path in report_paths:
            for case in json.loads(report_path.read_text())["cases"]:
                response = (ROOT / case["responsePath"]).read_bytes()
                request = (ROOT / case["requestPath"]).read_bytes()
                assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
                assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
                schemas[kind + "-response.schema.json"].validate(json.loads(response))
                if case.get("validRequest", True):
                    schemas[kind + "-request.schema.json"].validate(json.loads(request))
                count += 1
        raster_responses[kind] = count
    for report_path in args.paragraph_paths_report:
        for case in json.loads(report_path.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["paragraph-paths-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["paragraph-paths-request.schema.json"].validate(json.loads(request))
            paragraph_paths_responses += 1
    line_geometry_responses = 0
    for report_path in args.line_geometry_report:
        for case in json.loads(report_path.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["line-geometry-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["line-geometry-request.schema.json"].validate(json.loads(request))
            line_geometry_responses += 1
    line_shape_responses = 0
    for report_path in args.line_shape_report:
        for case in json.loads(report_path.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["line-shape-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["line-shape-request.schema.json"].validate(json.loads(request))
            line_shape_responses += 1
    for report_path in args.font_metrics_report:
        for case in json.loads(report_path.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["font-metrics-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["font-metrics-request.schema.json"].validate(json.loads(request))
            font_metrics_responses += 1
    for report_path in args.font_outlines_report:
        for case in json.loads(report_path.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["font-outlines-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["font-outlines-request.schema.json"].validate(json.loads(request))
            font_outlines_responses += 1
    for report_path in args.line_break_report:
        for case in json.loads(report_path.read_text())["cases"]:
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas["line-break-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas["line-break-request.schema.json"].validate(json.loads(request))
            line_break_responses += 1
    for report_path in args.itemization_report:
        for case in json.loads(report_path.read_text())["cases"]:
            kind = case["kind"]
            assert kind in ("itemization", "paragraph-shape")
            response = (ROOT / case["responsePath"]).read_bytes()
            request = (ROOT / case["requestPath"]).read_bytes()
            assert hashlib.sha256(response).hexdigest() == case["responseSha256"]
            assert hashlib.sha256(request).hexdigest() == case["requestSha256"]
            schemas[kind + "-response.schema.json"].validate(json.loads(response))
            if case.get("validRequest", True):
                schemas[kind + "-request.schema.json"].validate(json.loads(request))
            if kind == "itemization": itemization_responses += 1
            else: paragraph_responses += 1
    print(json.dumps({"textBodySources": text_body_sources, "textBodyResponses": text_body_responses, "textBodyRequests": text_body_requests, "textBodyEditRequests": text_body_edits, "sourceTextResponses": source_text_responses, "sourceTextEditRequests": source_text_requests, "sourcePageResponses": source_page_responses, "sourcePageRequests": source_page_requests, "sourcePlacementResponses": source_placement_responses, "sourcePlacementRequests": source_placement_requests, "presetExpansionResponses": preset_responses, "presetExpansionRequests": preset_requests, "gradientRasterResponses": gradient_responses, "gradientRasterRequests": gradient_requests, "fillColorResponses": fill_color_responses, "fillColorRequests": fill_color_requests, "fillColorEditRequests": fill_color_edits, "sourceEffectResponses": source_effect_responses, "sourceEffectEditRequests": source_effect_requests, "fillStyleInspections": fill_style_inspections, "fillStyleResponses": fill_style_responses, "fillStyleRequests": fill_style_requests, "fillStyleEditRequests": fill_style_edits, "sourceFillResponses": source_fill_responses, "sourceFillEditRequests": source_fill_requests, "nativePathResponses": native_path_responses, "nativePathRequests": native_path_requests, "geometryEvaluationResponses": geometry_eval_responses, "geometryEvaluationRequests": geometry_eval_requests, "geometryEvaluationEdits": geometry_eval_edits, "sourceGeometryResponses": source_geometry_responses, "sourceGeometryEditRequests": source_geometry_requests, "lineColorResponses": line_color_responses, "lineColorRequests": line_color_requests, "lineColorEditRequests": line_color_edits, "lineStyleResponses": line_style_responses, "lineStyleRequests": line_style_requests, "lineStyleEditRequests": line_style_edits, "sourceLineResponses": source_line_responses, "sourceLineEditRequests": source_line_requests, "pageRenderResponses": page_render_responses, "pageCompileResponses": page_render_responses, "pagePlacementResponses": raster_responses["page-placement"], "pathRasterResponses": raster_responses["path-raster"], "sceneRasterResponses": raster_responses["scene-raster"], "paragraphPathsResponses": paragraph_paths_responses, "paragraphLayoutResponses": paragraph_layout_responses, "lineGeometryResponses": line_geometry_responses, "lineShapeResponses": line_shape_responses, "fontOutlinesResponses": font_outlines_responses, "fontMetricsResponses": font_metrics_responses, "lineBreakResponses": line_break_responses, "itemizationResponses": itemization_responses, "paragraphResponses": paragraph_responses, "bidiResponses": bidi_responses, "cascadeResponses": cascade_responses, "unicodeResponses": unicode_responses, "textResponses": text_responses, "fontRequests": font_requests, "fontResponses": font_responses, "schemas": len(schemas), "positiveInputs": requests, "negativeMutations": len(mutations), "responses": responses, "opcResponses": opc_responses, "sourceResponses": source_responses, "sourceRequests": source_requests, "colorRequests": color_requests, "colorResponses": color_responses}))


if __name__ == "__main__":
    main()
