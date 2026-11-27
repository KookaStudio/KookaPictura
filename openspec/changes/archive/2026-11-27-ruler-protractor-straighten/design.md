# Design

## Context

The Ruler is a transient `PictureView` line backed by `pictura_core::Ruler`, shown by the tool handler and queried by options and Info. Canvas rulers are separate widgets in `CanvasScrollBars`; the intersection is currently inert. Arbitrary rotation already computes the correction from the primary line and performs a guarded document rotation with interpolation and history.

## Goals / Non-Goals

**Goals:** Extend existing interaction and document rotation paths, keep measurements transient, and maintain view isolation.

**Non-Goals:** Changing document pixels for ruler-origin movement or adding a general snap framework.

## Decisions

- Extend the ruler's view-state representation and bridge to carry the optional second arm. Reuse the existing constraint math and show both arms in the canvas overlay. This keeps tool switching and document isolation consistent with the existing line; a separate global protractor would not. Alt-drag uses the pressed endpoint as the vertex; a double-click uses the endpoint nearest the click (the end on a tie). The ordered primary line is unchanged by either gesture.
- Use the existing arbitrary document rotation operation and correction-angle helper for Straighten, including interpolation, refusal handling, refresh and history. Do not route through the modal dialog.
- Keep the origin in canvas view state and map ruler tick positions and coordinate readouts relative to it. The corner widget owns the drag and converts through the canvas transform. Place the origin at the released document point without snapping; target snapping is deferred with Snap / Snap To (#305).

## Risks / Trade-offs

- [Conflicting angle conventions] The long-form document's 3-4-5 example has the opposite sign from the existing `tools/ruler-tool` specification and implementation. The owner resolved this by preserving the existing signed single-line convention and using a nonnegative included angle for the protractor; the long-form example is historical.
- [Snap controls are stubs] Origin snapping to guides, grid and slices is deferred with the general View > Snap / Snap To work (#305).
- [Rotation can be refused] Use the established document-command result so failure leaves both document and measurement intact.
