# Design

## Context

The current preset manager stores application-wide pattern entries as native typed payloads and imports supported .pat files. Pattern Fill layers and Pattern Stamp already consume these entries; Edit Fill currently passes an index into built-in tiles. Selection coverage is the authority for validating a hard rectangle.

## Goals / Non-Goals

**Goals:** Share durable preset identities and tile snapshots across consumers; preserve the existing fill job, selection, history and memory behavior. Generate bounded deterministic layouts without an external script engine.

**Non-Goals:** Adobe Deco code or exact visual parity, third-party scripts, and .pat export. Existing supported imports remain intact.

## Decisions

Capture the selected visible image pixels in document coordinates and encode the tile using the existing native pattern preset payload. Validate that the selection mask is a solid rectangle before offering the name; do not use bounds alone because that accepts feathered and irregular selections. Append using PresetStore's atomic persistence and surface failures; keep the document unchanged. Use the existing content-derived `document_pattern_id` for the preset row so Pattern Fill and shape fill embedding accept the payload; the display name remains independently editable.

Pass a snapshot of the selected pattern payload to Edit Fill and Paint Bucket; builtin choices resolve through the same store. Extend the Bucket picker and bridge to accept native rows as well as built-ins. Record a self-contained, encoded tile snapshot and script choice in new Edit Fill action parameters, so replay does not depend on the current library or mutable names; preserve the existing index-only path for legacy actions. Apply pattern data using the existing fill transaction and alpha/blend policies so cancellation and undo remain normal.

Implement Brick, Cross Weave, Random, Spiral and Symmetry as distinct fixed-seed placement transforms in a bounded region, rasterizing only intersecting stamps. Brick staggers alternating rows; Cross Weave alternates perpendicular orientations; Random scatters placements deterministically; Spiral progresses outward along a spiral; Symmetry mirrors placements about a center. They are original approximations with no Adobe parity claim. Pattern orientation and placement are anchored consistently in document space; transparent tile samples are composited through the existing fill policy. Work and allocation are bounded by checked dimensions and existing budgets; if a layout cannot be admitted within those bounds, refuse it without a partial edit.

## Risks / Trade-offs

- Large selections or extremely small tiles can create excessive work → use checked dimensions, existing allocation budgets and a bounded iteration strategy.
- Document changes while a modal dialog is open → recheck the active document generation and selected region before publishing.
- Native preset payloads and legacy actions must remain readable → reuse existing codec and retain old command paths.
- Closed reference algorithms and CS6 screenshots are unavailable → describe layouts as approximations and keep oracle claims out of the UI.

## Migration Plan

Existing stored lists and legacy index-only actions remain readable; new action snapshots carry their own tile and script choice. Rolling back leaves newly added presets in the existing native library format and older actions unchanged.
