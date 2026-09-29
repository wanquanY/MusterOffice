# Advanced content and preservation

Animation, transitions, audio/video, SmartArt/diagrams and formulas are part of
the full product target. Their presence in a source file or a design document
does not mean every installed authoring, editing, rendering and playback path
supports them. Check the operation/profile and resulting diagnostics separately.

Keep timing, triggers, jump targets, media relationships and native editability
when editing. An object retained as source data has a different guarantee from
one parsed into native semantics. Never describe passthrough preservation as
successful playback or semantic editing. Avoid silently substituting a static
image for a requested editable formula, diagram or animated object.

When the required behavior is supported, use the engine's structured operations
and explicit playback state to inspect representative moments, including the
initial/final state and relevant triggers. Agent calls do not drive each frame;
the host integrates the kernel's playback output and its own media/UI controls.

If the discovered apply schema includes `setPresentationSequence`, use it for
ordered native effects. Supply click groups (`automatic` only for the first
group, otherwise `next`), sequential batches, and parallel effects within each
batch. Batch delay is additional to the preceding batch's complete duration;
effect delay is relative to its own batch. The compiler includes every effect's
repeat and speed when calculating those boundaries. Changing the sequence means
resubmitting the complete sequence in one revision-bound apply. Empty groups
clear the slide timeline. This operation replaces that slide's whole timeline;
do not use it to overwrite an imported or independently authored timing graph.

The result is an explicit timeline with native editorial groups. Use only effect
types present in the installed schema. If it includes `fade`, supply
`{"kind":"fade","target":"object-id","transition":"in"}` (or `"out"`) as the
effect inside a presentation sequence. This compiles the native preset together
with its visibility helpers: an initial entrance starts hidden, while an exit
hides the object when complete. Opacity applies to the whole object or group,
including overlapping fills, outlines, text and pictures. It is not a request
to rewrite every paint's alpha or drive frames from the Agent.

If the installed rotation schema includes `composition`, choose the angle domain
explicitly. Use `add` with a zero starting value for a relative spin that preserves
the effective underlying angle, for example
`{"kind":"rotation","target":"object-id","from":0,"to":21600000,"composition":"add"}`
for one full turn (60,000 units per degree). `layout` replaces the animation
offset relative to the object's original local angle; `absolute`, also the
legacy default when omitted, targets the final local angle itself. The kernel
owns overlap, removal and retained playback computation. Native by-only maps to
add, while from/to, from/by and to-only map to layout. Do not treat a returned
layout-based sampled rotation as an absolute angle or infer general native
additive/accumulate support from this bounded operation.

The lower-level `setTimeline` retains explicit graph semantics. A bare `fade`
behavior changes opacity only; it does not infer entrance visibility or add
preset helpers. The installed fade support does not imply arbitrary native
filters, custom progress curves, every animation preset or Office/WPS
compatibility for arbitrary timing. A sequential batch after an unbounded
batch is rejected. Do not reinterpret missing event targets, flatten interactive
dependencies, or promise that arbitrary node edges survive every traditional
editor's save operation.

When an operation is unavailable, preserve the original and state the specific
uncompleted requirement. Do not erase advanced content to make a basic export
pass or imply a complete editable deck has been delivered. Do not create a new
permission, task or media hosting system inside the skill to hide a missing
capability. The host supplies authorized media bytes and output delivery.
