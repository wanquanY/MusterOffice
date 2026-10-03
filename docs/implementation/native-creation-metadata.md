# Native slide and object creation metadata

Source visual coverage recognizes the exact Office 2010 `p14:creationId`
extension under `p:cSld/p:extLst/p:ext`, and the Office 2014 `a16:creationId`
extension under an object's `p:cNvPr/a:extLst/a:ext`. These identifiers are
nonvisual source metadata. The original OPC bytes remain unchanged.

Recognition is limited to the exact extension URI, namespace, parent path and
attribute vocabulary. Slide identifiers must be unsigned 32-bit values; object
identifiers must be braced hexadecimal GUIDs. Unknown children, siblings and
attributes still enter the visual diagnostics. This is not an opaque skip of
extension lists and does not authorize rendering unknown graphical content.

The common-slide-data extension is documented in Microsoft's
[Open XML SDK reference](https://learn.microsoft.com/en-gb/dotnet/api/documentformat.openxml.presentation.commonslidedataextension?view=openxml-2.7.2).

Regression coverage is in `crates/mo-pptx/tests/slide_creation_metadata.rs`.
It exercises retained bytes, malformed identifiers and unknown visual payloads;
the existing accessibility regression remains unchanged. Source tests and
`mo-presentation-source` strict clippy pass locally.

This source change is not yet included in a published integration release.
It removes one rejection for externally authored PPTX files. It does not add
complete native chart-page rendering: `source_page/layers.rs` still rejects a
GraphicFrame that is not a table. Rich templates with charts require that
separate kernel capability before product publication; removing charts or
suppressing the diagnostic is not part of this change.
