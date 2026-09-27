# Create, import and edit

Read the current `computation-request`, `computation-invocation` and
`computation-receipt` schemas from the configured engine. They define the draft
contract, permitted profiles, units, identifiers and nested document structure;
this skill does not duplicate them.

For creation, translate the user's purpose, source material and visual direction
into a coherent document. Choose dimensions, hierarchy, spacing and typography
for the content. Supply native text/shapes/images with stable object IDs. Use
fonts whose actual bytes and rights the host provides. An unavailable font is
an input problem, not permission to silently change typography.

For an existing presentation, import its actual bytes with an AssetInfo binding.
Keep the imported snapshot and source bindings. Inspect import diagnostics:
retained unknown XML is not the same as understood, editable or correctly
rendered content. Editing should preserve unaffected objects and source parts.

For modification, use the latest snapshot and revision returned by the caller's
last completed computation. Batch related edits into an atomic `apply` request.
Keep the user's unchanged content, notes and behavior; change only what the task
requires. A conflict needs the caller's current snapshot and a newly considered
edit, not blind replay against a different revision.

The current adapter returns the snapshot inside the receipt. The host saves and
reads it for subsequent calls; there is no server-side document ID lookup. Use
bounded host-side extraction of the needed objects for reasoning rather than
loading a large document or binary resource into the model context.

For export, supply the actual exporter identity from capabilities, the snapshot,
explicit font/image resources and the supported delivery settings. Use the
advertised profile and request schema; do not request a proposed full profile
that the installed build cannot execute. Inspect the resulting files and quality
report using the delivery reference before calling the user task complete.
