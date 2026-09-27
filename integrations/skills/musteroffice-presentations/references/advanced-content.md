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

When an operation is unavailable, preserve the original and state the specific
uncompleted requirement. Do not erase advanced content to make a basic export
pass or imply a complete editable deck has been delivered. Do not create a new
permission, task or media hosting system inside the skill to hide a missing
capability. The host supplies authorized media bytes and output delivery.
