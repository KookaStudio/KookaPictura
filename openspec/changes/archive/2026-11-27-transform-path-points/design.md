# Design

## Context

The path selection handler stores one component and one anchor locally; selection is discarded on tool deactivation. Work and saved paths and layer vector paths share editing operations. Existing transform sessions target layers and raster previews, and Again retains repeatable similarity transforms. The catalogue exposes eight inert path/point commands. Existing shape-layer transforms map anchors and handles through similarity, projective and warp maps.

## Decisions

- Keep sets of component and anchor indices in the active selection handler, exposed to routing but not stored in PSD. Shift-click toggles and marquee selects across components of the active path. An ordinary click replaces the selection. Clear on tool, target or document invalidation; do not let stale indices address a replacement path.
- Add a path target to the existing transform session for Work, saved, and editable layer vector-mask paths. Capture the selected geometry and complete target restoration data before preview. Recompute from the original on each update, apply the box and mode controls, and commit one history state or restore everything on cancel. The document has one live-session slot: integrate path backup and transform state into one transaction rather than starting a conflicting Path and Transform session.
- Keep the source frame for a path as floating-point curve or anchor bounds, rather than the raster session's enclosing integer rectangle; controls and repeat use its exact centre and projective and warp maps use the same frame.
- Map every selected component's anchors and direction points, preserving order, closure, point types, unselected geometry, vector-mask flags, and unrelated pixels. Similarity and skew use affine maps. Distort, Perspective and Warp map control points using the existing shape-layer convention: the resulting cubic is an approximation to the mathematically transformed curve. Reject invalid maps without changing geometry or history.
- Shape/vector-mask edits refresh affected rendering and retain mask flags. A non-translation transform of a live shape converts it to a regular path on commit, following shape-layer transform semantics. Preview and cancellation must restore both its original outline and live-shape metadata; undo restores the complete original target. Respect layer editability and position locks.
- Store the last successful nonidentity repeatable similarity transform per document, with the existing source-frame pivot-offset and translation convention. Again uses the current selected components or anchors and their shared selection bounds; selected-anchor bounds use anchor positions only, excluding direction handles, and component bounds use curve extents. Coincident or single-anchor bounds are degenerate and cannot be repeated; selected anchors carry both handles. Cancellation, identity and non-repeatable projective/warp commits do not overwrite the record. Clear a record when its document closes or is replaced, and never use another document's record. Selection invalidation and repeat-record lifetime are independent.

## Risks / Trade-offs

- Nonlinear maps of cubic control points are approximations; test coordinates against the same map used for shape layers, without claiming exact curve parity.
- Target changes, undo and document replacement can invalidate selection indices or an active preview. Cancel the transaction against its captured target before releasing it; refresh and clear stale selection.
- Preserve full layer mask metadata and live-shape state through preview, cancellation and history; path-only backup is insufficient.
