# Source-backed page append

Status: implemented; native SDK integration tests and format preservation tests pass.
Distribution and product acceptance are tracked separately.

Imported objects retain their original native addresses and opaque package parts.
Version 5 of the retained-source binding profile additionally permits independent
authored slides. It does not permit author operations to replace retained objects,
rewrite source masters/layouts, or strip provenance. Old profiles retain their
original validation semantics.

The presentation format owner partitions a mixed document into the retained graph
and authored slides. The existing retained-field planner verifies the complete
source projection after removing only the independent author partition. The
existing author writer validates and serializes that partition. A typed OPC
graph extension grafts its slide/master/layout/theme/media closure beneath a
collision-free namespace. Original opaque parts, original relationships and
embedded resources retain their bytes; presentation lists, their relationships
and content-type metadata receive bounded, namespace-aware child insertions.

The composed package is validated before becoming a presentation plan. Rendering
and export consume those same native bytes. This avoids claiming a visual result
from a different text projection. The native draft retains stable domain object
IDs for both partitions and remains editable. Reopening the exported PPTX uses
the ordinary retained importer. Signed source packages remain protected.

Verified by `mo-pptx/tests/source_append.rs`, `mo-xml/tests/insert_before.rs`,
and `mo-export-worker/tests/native_export/sdk.rs`. The worker test compares the
original pages pixel-for-pixel before and after append. These checks cover append after retained text edits, unchanged original part
hashes, relationship closure/collisions, reimport, rendered page order, repeatable
results, invalid provenance, cancellation and bounded package resources. Office
and WPS interoperability remains a separate acceptance step.
