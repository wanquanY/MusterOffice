# Inspect and deliver actual output

Read `result.json`, `files.json` and `inspection.json` from the completed output.
Check that the receipt matches the requested revision and profile, and verify
artifact byte lengths and hashes through the host's data channel. A tool error,
missing file, unverified claim or diagnostic is not converted to success because
an output directory exists.

View the actual slide previews with the host's image viewer. Inspect content
coverage, text clipping, line breaks, overlap, alignment, spacing, typography and
image placement. For requested dynamics, inspect supported playback samples as
well as the static view. Correct problems in the document and re-export a new
revision. Do not repair only the preview image while leaving PPTX content wrong.

Read the quality dimensions separately: structure, visual fidelity, editable
semantics, playback and target-application interoperability. Current development
builds report incomplete full-presentation and Office/WPS validation. Passing
XML or matching bytes is useful evidence, not proof of those broader properties.
If the host provides Office/WPS validation, report only the applications, versions
and edit/reopen operations actually exercised.

Return a host-accessible link or file handle to the produced PPTX, with a useful
name and material remaining limitations. `productCommitted: false` means the
engine computed files; the host still has to save/register/publish them according
to its own workflow. The skill does not authorize sharing with another person or
service, nor does it delete caller files when a plugin is upgraded or removed.
