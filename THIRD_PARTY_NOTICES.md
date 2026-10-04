# Third-party materials

The root [Apache-2.0 license](LICENSE) applies to original MusterOffice work,
including the original presentation examples contributed by the maintainer.
It does not replace the licenses of third-party code, data or dependencies.

This source repository keeps upstream notices alongside the relevant materials:

| Material | License and source records |
| --- | --- |
| Skia adaptations and upstream patch context; skcms and Khronos notices | [Notices](components/skia/THIRD-PARTY-NOTICES.txt), [source lock](components/skia/lock.json) |
| HarfBuzz component | [Source and license record](components/harfbuzz/lock.json); upstream source is fetched separately and its COPYING file must be retained |
| PNG, JPEG and zlib components | [Source lock](components/image-codec/lock.json), [license texts](components/image-codec/licenses/) |
| Unicode data | [Unicode license](crates/mo-unicode/data/LICENSE-UNICODE) |
| ECMA DrawingML preset shape data | [Copyright and permission](components/drawingml-presets/COPYRIGHT.txt), [provenance](components/drawingml-presets/README.md) |
| Unicode bidi dependency | [Copyright](components/unicode-bidi/COPYRIGHT), [MIT license](components/unicode-bidi/LICENSE-MIT) |
| Rust numeric dependencies | [Component record and licenses](components/rust-numeric/) |
| rustix | [Component record and licenses](components/rustix/) |
| base64 | [Component record and licenses](components/base64/) |
| MCP SDK | [License](components/rmcp/LICENSE-UPSTREAM.txt), [component record](components/rmcp/component.json) |
| I/O and optional HTTP runtime dependencies | [I/O notices](components/io-runtime/licenses/), [HTTP notices](components/http-runtime/licenses/) |
| Legacy SQLite host dependencies | [Component record](components/sqlite-host/component.json), [notices](components/sqlite-host/licenses/) |

Cargo and pnpm lockfiles identify additional dependencies fetched during builds;
those packages retain the notices in their upstream distributions. Compilers,
platform runtimes, downloaded fonts and other separately acquired build inputs
are not relicensed by this project. Binary redistributors must include the notices
required for their actual dependency closure.

The [template examples](examples/templates/README.md) contain original layouts,
fictional demonstration content and AI-generated illustrations. Their
[artwork provenance](examples/templates/artwork-provenance.json) records the
generation prompts and image digests. No font binaries or private Musterwork
application source are included in those examples.
