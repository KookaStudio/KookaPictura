# Design

## Context

The panel currently constructs disabled overlay widgets. `CloneSourceOptions` stores five per-slot sources and transforms; Clone Stamp establishes its source-to-destination offset at stroke start. Healing Brush has its own sampling path. The source mapping and brush footprint must agree with each tool's actual sampling behavior.

## Goals / Non-Goals

**Goals:** Provide non-destructive CPU-visible canvas feedback for both tools with controls synchronized with session state, including active stroke visibility and redraws.

**Non-Goals:** Cross-document sampling, source-overlay dragging or keyboard movement, Extended frame controls, and unrelated sampling or healing algorithm changes.

## Decisions

- Store overlay settings per source slot in session state, following the panel contract's proposed slot model. Defaults are Show/Auto Hide/Clipped/Invert off, opacity 100%, and Normal. Per-slot persistence and opacity default are unverified CS6 behavior; these are project-contract assumptions for review.
- Render the preview in the canvas feedback layer, using the same sampled surface and inverse transform/offset as the active tool. Before a first stroke, position the sampled anchor at the cursor; after an aligned offset exists, use the tool's established mapping. Update on pointer, tool, source, view, settings, and stroke changes. Clip to the actual brush footprint when requested. The preview itself does not alter the document, undo state, or paint sampling. A separate copied full-document image is unsuitable for large documents; restrict work to the visible region.
- A per-view worker samples source crops in bounded tiles; one worker runs at a time and superseded work is cancelled between tiles. A bounded overscanned crop is cached by document generation and revisions, layer path, sampling scope and adjustment option, and retained stroke snapshot identity. A new pointer position reuses the crop while covered; stale or cancelled results never land. The GUI polls without waiting and hides the preview until the current source is ready.
- Apply standard Normal, Darken, Lighten and Difference channel blend formulas to the preview, followed by opacity compositing; invert source RGB before blending while preserving alpha. This is an approximation: CS6 overlay blend math is unverified. On opaque matching source and destination, Normal plus 50% opacity and inversion yields gray. Respect transparent source pixels.
- Share the active source and overlay state with Healing Brush. Extend its offset-only sampling path to apply the active slot’s inverse scale, rotation and flips about the offset anchor, using the same mapping as its preview. Preserve the existing healing solve and unrelated retouch behavior. Keep overlay hidden without a valid source, outside the canvas or when another tool is active, and during a stroke when Auto Hide is on.

## Risks / Trade-offs

- [Preview disagrees with actual sampling, especially non-aligned strokes or transforms] → exercise both tools and aligned/non-aligned mapping with deterministic image tests.
- [Large previews cause latency] → draw only the visible source region and avoid full-surface copies; invalidate when source content changes.
- [Unverified Adobe behavior] → label blend and persistence choices as approximations and keep them reviewable; do not claim oracle parity.
- [Healing Brush currently has independent source state and offset-only sampling] → integrate the shared source model and transformed sampling; test actual healed output as well as the preview and slot selection.
