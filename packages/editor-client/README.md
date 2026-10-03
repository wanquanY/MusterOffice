# Presentation editing computation

`PresentationEditor` is a thin, synchronous facade over the matching verified
Rust/WASM module. Run it in the receiving product's Worker. It creates no UI,
history store, account, network connection or durable editing session.

The pinned playback runtime also returns `runtime.editor`, sharing its already
initialized kernel. `initialize` verifies a document, `prepare` computes a new
atomic candidate, `prepareHistory` computes a guarded reversal, and
`prepareText` replaces grapheme-aligned ranges across runs/paragraphs or patches
selected character declarations without resolving unrelated inherited styles. `placements`
uses the renderer's existing author placement engine; it is not yet a complete
hit-test or caret API. `segment` returns the kernel's pinned grapheme boundaries
with scalar, UTF-8 and UTF-16 offsets. `paragraphInteraction` takes explicit font
bytes and a shaping component; caret, hit and selection queries share the
renderer’s paragraph line plan and exact Q32 pen. It returns grapheme boundaries
and separate upstream/downstream positions at bidi or soft-wrap boundaries.
This paragraph API does not yet provide page-level transformed/clipped hits or
IME handling. See [paragraph interaction](../../docs/implementation/paragraph-text-interaction.md).
Source-backed documents retain their
existing operation restrictions and require their source placement path.

The receiving host authorizes all reads and writes, resolves original history
material from its own durable receipts, verifies resources, and commits the
candidate and receipt with compare-and-swap. A successful calculation is not a
saved draft or published document. The host should accept an original history
reference from a user, never trust a client-provided history snapshot as proof.

History recomputes the original transaction before reversing its changes. Each
changed declaration must still equal the expected value; unrelated declarations
are retained. Conflicting declarations or invalid references reject the whole
candidate. Undo and redo generate new revisions. Saving a checkpoint need not
discard history. The original checkpoint's resources must remain retained while
that checkpoint is reachable from the host's history.

Text commands expand to ordinary semantic transactions, so validation, history
and source-backed restrictions stay in the same engine. Persist both the original
command (with its intent digest) and expanded transaction. Exact scalar change
maps preserve range lineage; the returned display selection is on a resulting
grapheme boundary, including when an edit forms a new emoji sequence. This API
currently edits authored shape text, not table cells or retained source runs.

Validation uses real native and WASM builds:

```sh
cargo test --locked -p mo-presentation-edit
cargo build --locked -p mo-cli
cargo build --locked --release -p mo-wasm --target wasm32-unknown-unknown
# Use the matching wasm-bindgen version from Cargo.lock to generate nodejs
# bindings in .codex-work/native-editor/wasm-node (CommonJS package scope).
pnpm test:editor-client
```

`MUSTEROFFICE_EDITOR_WASM` and `MUSTEROFFICE_EDITOR_NATIVE` can select explicit
build outputs. The test fails if either is absent. This evidence does not
qualify browser rendering, text caret/IME interaction, host persistence or
PowerPoint/WPS interoperability.
