# Fixed native media component

Development component, not yet a released SDK or completed editing engine. Original `mo_media*` / `mo_encode*` wrappers are Apache-2.0; FFmpeg remains under its applicable upstream licenses.

## Source and distribution boundary

- FFmpeg **9.0.2**, released 2026-09-18, selected from the [official download page](https://ffmpeg.org/download.html). Archive SHA-256: `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e`; 12,040,788 bytes. Full coordinates are in [lock.json](lock.json).
- Archive signature was verified against the official release key fingerprint `FCF986EA15E6E293A5644F10B4322F04D67658D8`. The armored [signature](source-signature.asc) and [public key](release-key.asc) are retained. GnuPG's local web-of-trust warning is distinct from the verified signature/fingerprint; the key was obtained from the official HTTPS source.
- This profile explicitly disables GPL, version3, nonfree, networking, protocols, programs, autodetected dependencies, filters and device capture. Shared avformat/avcodec/avutil/swscale/swresample are built from the pinned archive. There is no lookup of Homebrew/system codec libraries.
- [LGPL-2.1 text](COPYING.LGPLv2.1) and [upstream license inventory](LICENSE.md) are retained. Future binary packages must provide the exact corresponding source archive, per-file notices, configure/build records, the [fixed upstream modification](patches/0001-videotoolbox-bounded-reorder-depth.patch) and corresponding modified sources, and required replacement/relinking provisions. Dynamic linkage alone does not discharge distribution obligations. See the [official legal guidance](https://ffmpeg.org/legal.html).
- Mac builds optionally compile the system VideoToolbox/AudioToolbox interfaces; the current decoder ABI selects software decoding and makes no hardware acceleration claim. System frameworks are platform dependencies, not vendored under Apache-2.0.

## Reproduction

Fetch the archive from the URL in `lock.json`, verify the source digest and optionally reproduce signature verification in a dedicated keyring. Builders never download implicitly. Python 3.13, clang, make and an appropriate assembler on x86 are required. Use a fresh directory for the libraries stage:

```sh
python3 tools/components/build-ffmpeg.py --stage libraries \
  --archive /path/to/ffmpeg-9.0.2.tar.xz --directory .codex-work/ffmpeg-native
python3 tools/components/build-ffmpeg.py --stage bridge --directory .codex-work/ffmpeg-native
python3 tools/verification/media-native-check.py \
  --component .codex-work/ffmpeg-native --output .codex-work/media-native-check
```

The libraries stage verifies the archive before extraction, applies the fixed zero-fuzz modification and records its digest, then records configure commands, compiler/platform, library and installed header digests. The bridge stage verifies that record and all those digests before compiling our wrapper, then records its C/header sources and binary digest. A sanitized build uses `--sanitize` in **both stages**, a separate fresh directory, and the same flag on the check script. This instruments C code with ASan/UBSan, including the upstream C implementation; assembly/platform frameworks are not thereby instrumented. Sanitizers are verification profiles, never release binaries.

The check script uses an external FFmpeg/ffprobe CLI **only to generate and independently inspect synthetic fixtures**. Its exact version and commands are recorded; its codec libraries are not linked into the component. The optional software encoders installed with that test utility are not distribution dependencies of this component.

`tools/verification/current.py --groups media --ffmpeg-dir <directory> --output <new-directory>` runs the real-media gate. The default CI now builds the fixed component and invokes that group. Linux CI configuration is present; the current local evidence is macOS arm64, not a completed Linux/Windows matrix.

## Current ABI behavior

`mo_media.h` defines ABI 1, opaque single-thread-affine session ownership, explicit limits and stable wrapper status codes. No FFmpeg struct layout escapes it.

- Input is a host-owned `read_at` callback over a fixed byte length plus cancellation callback. Reads/seeks are bounded to that resource. Embedded references cannot open another URL or path: custom I/O, disabled protocols/network and a rejecting `io_open` callback enforce that boundary.
- Header probing does not invoke `avformat_find_stream_info`, which can open decoders before our per-decoder limits are installed. Unknown header values remain unknown. A future Rust probe layer can explicitly decode selected streams under the same limits to obtain missing parameters.
- Each session selects one audio or video stream. Parameters carry stream timebase, optional start/duration, codec, channel layout, SAR, color metadata and container display matrix. Separate sessions supply video and audio without racing a shared demux cursor.
- `next` follows the [documented send/receive state machine](https://ffmpeg.org/doxygen/trunk/group__lavc__encdec.html), retains a pending packet, sends EOF once, drains delayed frames and never stops because a packet PTS crossed a requested boundary. Unexpected EAGAIN on both sides is an error, not a retry/sleep loop. Fatal decode/I/O/budget errors poison the session until close.
- `seek` targets an earlier random-access point and flushes codec state; its generation increases. It does **not** claim to return the exact target frame. The Rust layer must decode/discard to the desired presentation interval and honor before/after semantics.
- Frame metadata reports actual PTS/timebase/duration, dimensions, audio samples, source pixel/sample format, color tags, SAR, alpha mode and interlace flags. Missing timestamps/durations are explicitly marked, not synthesized from average FPS.
- Active raw planes are copied with caller-provided capacity/stride checks; decoder padding is never exposed. No implicit conversion to 8-bit, SDR, mono or system colors occurs. Palette/hardware surfaces currently need a different adapter and fail explicitly. A cancelled copy may have partially written its destination; the caller must discard that destination.
- AAC priming/tail discard is handled by the selected decoder metadata. The fixture asserts the independent known 3-second/48k source extent, including a final 640-sample block. The older local reference CLI retains 384 tail samples; only that reference is explicitly trimmed to the known source extent for comparison. Our decoded result is not trimmed to make the test pass.

Limits cover input/read bytes, stream count, pixel/sample/channel limits, active frame bytes and decode-loop work. They are not a hard bound on every private vendor allocation or the duration of a single codec call. The subsequent [Rust adapter](../../docs/implementation/video-media-native.md) now provides RAII, panic boundaries, owned planes and ABI layout checks. A [killable worker](../../docs/implementation/video-media-worker.md) now supplies bounded resource IPC, deadlines and startup process limits; full platform syscall sandboxing and product integration remain pending. The API must not be used to imply that arbitrary hostile media is safe in the UI process.

## Encoding ABI

`mo_encode.h` and `mo-media-ffmpeg::Encoder` now provide typed H.264 VideoToolbox/AAC MP4 and FFV1/float PCM MP4 audit profiles, exact finite clocks, bounded interleaving and host-only candidate writes. Hardware capability is checked by opening the actual profile; no software or format substitution occurs. The pinned VideoToolbox modification fixes hierarchical H.264 frame timestamps using the actual SPS reorder bound. See [native encoding implementation and evidence](../../docs/implementation/video-native-encoding.md) for profiles, short clips, color interpretation, error handling, sanitizers and remaining product integration. The derivative modification remains under FFmpeg's LGPL conditions; it is not relicensed as an original Apache wrapper.

## Evidence and remaining work

macOS arm64 local checks: 77 actual H.264 frames with VFR gaps and decode/presentation reordering, exact frame PTS/active-plane MD5 matches against the independent decoder, forward/backward seek with generation changes, 13-byte short reads, EOF/drain, cancellation, invalid reuse, too-small output capacity and input/read/frame/pixel/work budgets. Explicit BT.709 tags are checked in the fixture before verifying decoder metadata. AAC: 144,000 samples with continuous timestamps; maximum PCM error against the independent decoder below `1e-5`, RMS below `1e-6`. Normal and ASan/UBSan profiles both pass.

Local reports live in ignored `.codex-work/media-native-checked/report.json` and `media-native-sanitized-current/report.json`; they contain source, compiler, artifacts, commands and reference versions. Release-profile shared libraries plus wrapper total 7,392,248 bytes on this build. This excludes the Rust worker, fonts, GPU runtime, other optional codecs and platform frameworks; it is not the final product download/memory size or performance result.

The media verification group also runs `media-rust-check.py` against that run's generated fixture: Rust ownership/cancellation tests and real pixel/PCM/timestamp comparisons. Configure `MO_FFMPEG_LIB_DIR` to this component's root to compile `mo-media-ffmpeg`. The native crate checks the exact component records, library loader aliases and all public C/Rust field layouts before use.

The media group now builds a pinned development worker bundle and executes real host/worker decode, crash/deadline/reaping and bundle integrity tests. The host binary is checked to have no codec linkage. This is not a signed release or an OS syscall sandbox.

Remaining: platform sandboxing, source indexing and compile-plan integration, browser I/O, hardware frames, full color interpretation/conversion, additional codecs, audio processing, project export and encoding worker integration, semantic editing completion and product UI. C/Rust decode checks do not close those acceptance items.
