# Real browser playback verification

`../browser-playback.py` launches an explicitly selected installed Chromium
browser using a new temporary profile. It serves only pinned development package
bytes, owned fixture inputs and this harness through an ephemeral loopback
server. It never accesses existing tabs, a user profile or arbitrary file URLs.
The browser executable is not installed/downloaded by the runner.

```sh
python3 tools/verification/browser-playback.py \
  --output .codex-work/browser-check-new \
  --bundle /absolute/path/to/verified-sdk --pin EXTERNAL_MANIFEST_SHA256 \
  --author .codex-work/playback-render/product.json \
  --source .codex-work/source-playback/product.json \
  --native-reference .codex-work/sdk-playback/reference-02 \
  --browser '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
```

All historical record bytes/digests are verified before serving. A native SDK
reference supplies full pixels and frame metadata; the server compares actual
browser bytes and parsed metadata, not only frame names or declared hashes.
Outputs are new-only. The same package is checked again after execution.

Ordinary execution has no cross-origin isolation headers and uses no shared
memory. A separate `--isolated` run enables COOP/COEP solely so the fault probe
can observe a SharedArrayBuffer execution counter across termination. Documents,
fonts and SDK frame transport still use independent transferred ArrayBuffers.
No SharedArrayBuffer is added to the computation interface.

Faults enter the actual Rust owner's raster callback. Short and long busy loops
separate logical cancellation from physical execution. Chromium may let short
work finish after terminate and only interrupt long work later. The report
preserves the observed tail and rejects a long callback that survives the
termination observation window; it does not certify the 200 ms / 2 s product
performance targets. Counter sampling/stability is test evidence, not a public
termination acknowledgement, CPU/RSS benchmark or a guarantee for other engines.

The harness additionally exercises backpressure, expired/active abort signals,
deadline/close, late-result suppression, malformed/correlated replies, clone and
script failures, actual invalidated raster component, recoverable errors,
signal listener cleanup and creation of fresh Workers. The independent reference
workload checks 10 retained owners, exact generation, source/font detachment and
no repeated shaping/font upload. It is not a product Viewer/Player or UI test.
