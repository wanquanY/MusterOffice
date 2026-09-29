"""Measure complete native exports of repeated, caller-owned compose slides.

This is a local measurement harness, not the mixed-content W-S10/W-S40 gate.
It never changes the supplied content, fonts, render settings, or kernel limits.
Python standard library only; macOS/Linux ps is used for sampled process RSS.
"""
import argparse
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import signal
import statistics
import struct
import subprocess
import threading
import time
import xml.etree.ElementTree as ET
import zipfile


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def identity(path):
    data = Path(path).read_bytes()
    return dict(byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


def measured_run(command, folder, timeout, interval):
    """RSS sums include shared pages; observations are not a true physical peak."""
    done = threading.Event()
    outcome = {}
    samples = []
    with (folder / "stdout.log").open("wb") as stdout, (folder / "stderr.log").open("wb") as stderr:
        start = time.perf_counter()
        process = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)

        def wait():
            outcome["exitCode"] = process.wait()
            outcome["wallSeconds"] = time.perf_counter() - start
            done.set()

        waiter = threading.Thread(target=wait)
        waiter.start()
        try:
            while not done.is_set():
                tick = time.perf_counter()
                if tick - start > timeout:
                    raise TimeoutError("Owned export process group exceeded measurement timeout")
                rows = subprocess.check_output(["ps", "-axo", "pid=,ppid=,pgid=,rss="], text=True, timeout=5)
                # Only our new process group enters the report. No command lines are collected.
                processes = [tuple(map(int, row.split())) for row in rows.splitlines()]
                owned = [row for row in processes if row[2] == process.pid]
                if owned:
                    samples.append(dict(elapsedSeconds=tick-start,
                                        processCount=len(owned),
                                        rssSumBytes=sum(row[3] for row in owned)*1024,
                                        rootRssBytes=sum(row[3] for row in owned if row[0] == process.pid)*1024))
                done.wait(max(0, interval - (time.perf_counter() - tick)))
        finally:
            if not done.is_set():
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            waiter.join()
    write(folder / "rss-samples.json", samples)
    outcome.update(sampleCount=len(samples),
                   maxSampledRssSumBytes=max((s["rssSumBytes"] for s in samples), default=0),
                   maxSampledRootRssBytes=max((s["rootRssBytes"] for s in samples), default=0),
                   maxObservedProcessCount=max((s["processCount"] for s in samples), default=0))
    return outcome


def invoke(args, folder, invocation, inputs, export=False):
    folder.mkdir()
    spool = folder / "spool"
    spool.mkdir(mode=0o700)
    write(folder / "invocation.json", invocation)
    command = [str(args.cli), "compute", str(folder / "invocation.json"),
               str(inputs), str(spool), str(folder / "output")]
    if export:
        command.extend([str(args.worker), args.worker_sha256])
    metric = measured_run(command, folder, args.timeout, args.sample_interval)
    if metric["exitCode"] != 0:
        return metric, None
    output = folder / "output"
    summary = read(folder / "stdout.log")
    result_id = identity(output / "result.json")
    assert summary["resultSha256"] == result_id["sha256"]
    assert int(summary["resultByteLength"]) == result_id["byteLength"]
    assert summary["productCommitted"] is False
    # The execution registry lock is the only permitted residual temporary file.
    residual = sorted(str(p.relative_to(spool)) for p in spool.rglob("*") if p.is_file())
    assert residual == ([".mo-executions-v1/registry.lock"] if export else []), residual
    return metric, read(output / "result.json")["result"]


def verify_export(output, pages, shapes_per_page, text_frames, width):
    report = read(output / "inspection.json")
    assert report["pages"] == pages
    layout = report["layoutDiagnostics"]
    assert layout["measuredPages"] == pages and layout["unmeasuredPages"] == 0
    assert layout["measuredFrames"] == pages * text_frames
    assert layout["affectedFrames"] == layout["omittedFindings"] == 0
    assert not layout["findings"]
    overlaps = layout["textOverlaps"]
    assert overlaps["measuredPages"] == pages and overlaps["unmeasuredPages"] == 0
    assert overlaps["uncheckedPairs"] == overlaps["intersectingPairs"] == overlaps["omittedFindings"] == 0
    assert not overlaps["findings"]
    assert overlaps["checkedPairs"] == pages * text_frames * (text_frames-1) // 2
    assets = read(output / "files.json")
    stable = []
    pptx_size = None
    preview_bytes = 0
    for item in assets:
        file = output / item["file"]
        observed = identity(file)
        assert observed == dict(byteLength=int(item["asset"]["byteLength"]), sha256=item["asset"]["sha256"])
        role = item["asset"]["role"]
        if role in ("preview", "pptx"):
            stable.append(dict(role=role, **observed))
        if role == "preview":
            png = file.read_bytes()
            assert png[:8] == b"\x89PNG\r\n\x1a\n"
            assert struct.unpack(">I", png[16:20])[0] == width
            preview_bytes += len(png)
        if role == "pptx":
            pptx_size = observed["byteLength"]
            with zipfile.ZipFile(file) as package:
                assert package.testzip() is None
                slides = [n for n in package.namelist()
                          if n.startswith("ppt/slides/slide") and n.endswith(".xml")]
                assert len(slides) == pages
                ns = {"p": "http://schemas.openxmlformats.org/presentationml/2006/main"}
                for slide in slides:
                    xml = ET.fromstring(package.read(slide))
                    assert len(xml.findall(".//p:sp", ns)) == shapes_per_page
                    assert not xml.findall(".//p:pic", ns)
    assert pptx_size is not None
    assert len(stable) == pages + 1
    return dict(assetsVerified=len(assets), totalAssetBytes=sum(int(a["asset"]["byteLength"]) for a in assets),
                pptxBytes=pptx_size, previewBytes=preview_bytes, layoutDiagnostics=layout), stable


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("compose", "settings", "inputs", "cli", "worker", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--worker-sha256", required=True)
    parser.add_argument("--pages", type=int, nargs="+", default=[10, 40])
    parser.add_argument("--slide-index", type=int, default=0)
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--sample-interval", type=float, default=0.1)
    parser.add_argument("--timeout", type=float, default=120)
    args = parser.parse_args()
    assert args.trials >= 1 and all(0 < n <= 80 for n in args.pages)
    assert args.sample_interval >= 0.02 and args.timeout > 0
    for name in ("compose", "settings", "inputs", "cli", "worker", "output"):
        setattr(args, name, getattr(args, name).resolve())
    assert identity(args.worker)["sha256"] == args.worker_sha256
    compose = read(args.compose)
    settings = read(args.settings)
    assert compose["kind"] == "compose"
    assert settings["renderer"]["implementationSha256"] == args.worker_sha256
    source = compose["presentation"]["slides"][args.slide_index]
    # This workload deliberately covers flat text/rectangle compose elements only.
    assert all(set(e) <= {"id", "frame", "fill", "text", "stroke"} for e in source["elements"])
    args.output.mkdir()
    empty = args.output / "empty-inputs.json"
    write(empty, [])
    normalized_inputs = read(args.inputs)
    input_identities = []
    for item in normalized_inputs:
        item["file"] = str((args.inputs.parent / item["file"]).resolve())
        actual = identity(item["file"])
        descriptor = item["info"]["descriptor"]
        assert actual["sha256"] == descriptor["sha256"] and actual["byteLength"] == int(descriptor["byteLength"])
        input_identities.append(dict(id=item["info"]["id"], **actual))
    inputs = args.output / "inputs.json"
    write(inputs, normalized_inputs)
    environment = dict(platform=platform.platform(), architecture=platform.machine(),
                       python=platform.python_version(), loadAverageAtStart=os.getloadavg(),
                       cache="OS caches not flushed; one excluded warmup per workload; every compose/export starts fresh processes",
                       timing="CLI start through normal exit; includes worker startup, shaping, rendering, PNG/PPTX generation, SDK output verification and local publication; excludes harness post-checks",
                       rss="Maximum sampled sum of RSS for the CLI process group, including worker; shared pages may be double counted; short peaks may be missed; not incremental/product/physical peak memory",
                       sampleIntervalSeconds=args.sample_interval)
    if platform.system() == "Darwin":
        environment["cpu"] = subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()
        environment["memoryBytes"] = int(subprocess.check_output(["sysctl", "-n", "hw.memsize"], text=True))
    report = dict(format="musteroffice.native-text-shape-export-scaling/1", environment=environment,
                  programs=dict(cli=identity(args.cli), worker=identity(args.worker)),
                  inputs=dict(compose=identity(args.compose), settings=identity(args.settings), assets=input_identities),
                  sourceSlideIndex=args.slide_index, sourceElements=len(source["elements"]),
                  sourceTextFrames=sum("text" in e for e in source["elements"]),
                  trials=args.trials, records=[], summary={}, failures=[],
                  limitations=["Repeated text and rectangle page, not mixed W-S10/W-S40 or complex-content acceptance.",
                               "Local release process measurement, not Agent latency, installer size, Office/WPS compatibility or global peak RSS.",
                               "One warmup then the declared number of measured trials; P95 only for at least 30 trials, using nearest rank. Other applications/services remain running."])
    for pages in args.pages:
        action = copy.deepcopy(compose)
        action["presentation"]["slides"] = []
        for index in range(pages):
            slide = copy.deepcopy(source)
            slide["id"] = f"slide:scale-{index}"
            slide["name"] = f"Scale {index+1}"
            for element_index, element in enumerate(slide["elements"]):
                element["id"] = f"scale-{index}-{element_index}"
            action["presentation"]["slides"].append(slide)
        request = dict(contractVersion="musteroffice.computation/1-draft",
                       profileId="presentations-author-model-v01-draft", requestId=f"scale-compose-{pages}", action=action)
        compose_metric, created = invoke(args, args.output / f"compose-{pages}", dict(request=request), empty)
        if created is None:
            raise RuntimeError(f"Compose failed; see compose-{pages}/stderr.log")
        snapshot = created["snapshot"]
        request.update(profileId="presentations-pptx-resource-delivery-v1-draft", requestId=f"scale-export-{pages}",
                       action=dict(kind="export", documentId=snapshot["document"]["id"],
                                   baseRevision=snapshot["revision"], settings=settings))
        expected = None
        for iteration in range(args.trials+1):
            folder = args.output / f"export-{pages}-{iteration}"
            metric, result = invoke(args, folder, dict(request=request, snapshot=snapshot), inputs, True)
            record = dict(pages=pages, iteration=iteration, warmup=iteration == 0, **metric)
            if result is None:
                report["failures"].append(dict(**record, diagnostic=read(folder / "stderr.log")))
                write(args.output / "report.json", report)
                raise RuntimeError(f"Export failed; see {folder.name}/stderr.log")
            checked, stable = verify_export(folder / "output", pages, report["sourceElements"],
                                            report["sourceTextFrames"], settings["delivery"]["previewWidth"])
            if expected is None:
                expected = stable
            assert stable == expected, "Output bytes changed between identical invocations"
            record.update(checked)
            report["records"].append(record)
            write(args.output / "report.json", report)
            print(json.dumps({k: record[k] for k in ("pages", "iteration", "wallSeconds", "maxSampledRssSumBytes", "pptxBytes")}), flush=True)
        measured = [r for r in report["records"] if r["pages"] == pages and not r["warmup"]]
        report["summary"][str(pages)] = dict(compose=compose_metric,
            exportWallSeconds=dict(median=statistics.median(r["wallSeconds"] for r in measured),
                                   minimum=min(r["wallSeconds"] for r in measured), maximum=max(r["wallSeconds"] for r in measured)),
            maxSampledRssSumBytes=max(r["maxSampledRssSumBytes"] for r in measured),
            deterministicPptxAndPreviews=expected)
        if len(measured) >= 30:
            report["summary"][str(pages)]["exportWallSeconds"]["p95NearestRank"] = sorted(
                r["wallSeconds"] for r in measured)[math.ceil(0.95 * len(measured)) - 1]
        write(args.output / "report.json", report)


if __name__ == "__main__":
    main()
