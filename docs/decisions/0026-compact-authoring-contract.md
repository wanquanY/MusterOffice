# Compact authoring contract

Status: implemented for development integration, 2026-10-01. No product release,
Office/WPS roundtrip, or agent performance acceptance is implied.

The public `operation::compose::authoring::AuthoringAction` contract provides
create, append and targeted edits using the existing SlideContent and PlainText
inputs. `bind` consumes an explicit document identity, optional exact base and
host-authorized Resource metadata, compiling to existing DocumentAction and
OperationEntry values. The native Document remains the only mutable model.

The host owns content authorization, metadata lookup, durable draft identity and
CAS, configured fonts, persistence and publication. The kernel never resolves a
product content ID or guesses a newer revision. `picture_resources` enumerates
selected logical resources; it neither grants permission nor fetches bytes.

SetFrame changes only the target transform. ReplaceText explicitly replaces the
whole text body using the creation grammar; it is not a rich-text preservation
patch. Movement preserves IDs and deletions reject dependencies. Advanced
format-preserving text operations remain available through the native edit API.
No page-rebuild shortcut silently discards references, notes or timelines.

Accessibility title/description have empty-string defaults while decorative
intent remains explicit when the object is supplied. Existing documents still
serialize their full semantics. Compact element decoding chooses picture vs
shape from the picture field and retains nested field diagnostics rather than
collapsing alignment/resource errors into an untagged-enum mismatch.

The generated authoring-action schema is part of public discovery. Source schema
and TypeScript generation are checked with the ordinary contract generator.
Tests exercise fixed revision conflicts, preserved object contents on geometry
edits, omitted decorative mechanical fields, invalid alignment, unauthorized
resource absence, and existing compose/append semantics. The development SDK is
assembled into immutable registry packages, with the existing unchanged worker
and playback bytes explicitly paired. Host integration independently exercises
14 pages, authorized picture references, local repair and two actual exports.
