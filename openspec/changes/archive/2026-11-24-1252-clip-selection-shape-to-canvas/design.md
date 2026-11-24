# Design

## Context

See `proposal.md` for the motivation. The production caller is `ImageView::setSelectionShapePreview`, and six Qt Test cases pin the current behavior. The C++ directory is not fully covered by the code graph, so the exact source and flagged test ranges are the evidence for this design.

The existing `tools/selection-outline-performance` capability defines the user-visible clipping behavior. This change adds engine ownership and a parity contract for the pinned cases and differential corpus without changing that tested behavior.

## Goals / Non-Goals

**Goals:**

- Keep one production clipping implementation, in `pictura-select`.
- Preserve the six pinned cases and the current C++ signature.
- Compare topology independently of vertex order and harmless collinear splits.
- Keep Qt types out of the engine and generated cxx-qt headers out of non-adapter C++ files.

**Non-Goals:**

- Changing selection rasterization, commit behavior, pan, zoom, or ants painting.
- Retiring the Qt reference in tests.
- Supporting curves. The input is already flattened `QPolygonF` data.

## Decisions

### D1. Use `i_overlay` 9.2.0

`i_overlay` currently provides `SingleFloatOverlay`, `OverlayRule::Intersect`, and `FillRule::EvenOdd`. Its default feature set is empty, it has no Qt dependency, and its license is MIT OR Apache-2.0. The four required support crates, `i_float`, `i_shape`, `i_tree`, and `i_key_sort`, are MIT-licensed. The optional Rayon feature stays disabled.

The adapter calls `overlay_as::<i64>` so the fixed-point backend keeps sub-micro precision at document-sized coordinates. A hand-written arrangement clipper would need crossing insertion, overlap handling, face classification, contour assembly, degeneracy cleanup, and the same differential corpus. That is more code to own than the dependency footprint, and the pinned self-overlap case is exactly where a polygon-only Sutherland-Hodgman clip fails.

Alternative considered: keep a private arrangement implementation in `pictura-core`. Rejected for this change because it moves the topology risk into the project instead of using a maintained boolean engine. Alternative considered: add the dependency to `pictura-app`. Rejected because the goal is a reusable Qt-free engine API.

### D2. Put the API in `pictura-select::clip`

The public function takes borrowed loops and a floating-point canvas size and returns owned loops:

```text
clip_selection_shape_to_canvas(&[Vec<(f64, f64)>], (f64, f64)) -> Vec<Vec<(f64, f64)>>
```

The engine keeps the current fast paths before calling the boolean library:

1. A canvas with non-positive or non-finite width or height returns the input unchanged.
2. A loop with fewer than three points is copied unchanged.
3. A loop whose bounds are inside the canvas is copied unchanged.
4. A loop wholly outside the canvas is dropped.
5. A non-finite loop is copied unchanged rather than reaching an infallible library API that panics on invalid bounds.
6. Otherwise, the loop and canvas rectangle are intersected with the even-odd rule.
7. If the boolean library returns no contours, a closed loop or a loop with nonzero signed area is copied unchanged.
8. An open zero-area loop is clipped as a string path with the canvas boundary included. If that returns no path, the original loop is copied unchanged.
9. Contours with fewer than two points are discarded.
10. Using pre-snap coordinates, contours are spliced only when they share an exact endpoint. Near endpoints never cause a splice.
11. Coordinates within `1e-6` of a canvas edge are snapped to that exact edge, and every returned vertex is clamped inside the canvas.

The explicit closing vertex is optional on input. Fast paths preserve it exactly; the boolean path ignores a repeated closing vertex. The output follows `i_overlay`'s unclosed contour convention; `QPolygonF` closes it when drawn.

### D3. Compare topology with a `1e-6` document-pixel gate

Preserved fast paths use exact ordered equality, including duplicate points and an explicit closing vertex. This applies to invalid or empty canvases, short traces, inner loops, and the empty-intersection fallback.

For filled intersections, the Qt test keeps a local copy of the complete old clipping function as its oracle. `QPainterPath::intersected` appears only in that copy's boolean branch. The comparison helper removes duplicate and collinear vertices, splits implicit edge crossings, and compares unordered undirected boundary segments with a Euclidean endpoint tolerance of `1e-6`.

This permits harmless differences in traversal order, loop order, collinear segmentation, and whether a crossing is explicit. It still fails when a segment is missing, an extra segment appears, a hole changes, or an endpoint moves beyond the gate. A teeth test shifts a canonical loop by `0.5e-6` and `2e-6` to prove the comparator accepts the first and rejects the second.

Rust tests replay the six traces and the differential inputs against hand-computed boundaries. A large-coordinate case checks exact border snapping and sub-micro placement of non-border intersections.

### D4. Use the existing selection bridge and API adapter

`impl_selection/bridge.rs` gains this shared struct and free function. Points are flat `(x, y)` pairs, and lengths count points rather than coordinate scalars.

```rust
struct SelectionClipResult {
    valid: bool,
    loop_lengths: Vec<u32>,
    points: Vec<f64>,
}

fn selection_clip_shape_to_canvas(
    loop_lengths: &[u32],
    points: &[f64],
    canvas_width: f64,
    canvas_height: f64,
) -> SelectionClipResult;
```

The implementation in `bridge/imp.rs` requires an even point count, a checked length sum equal to `points.len() / 2`, and no length larger than the remaining point pairs. Overflow, truncation, or trailing points return `valid = false` with empty vectors. Non-finite point coordinates are not malformed; the engine's existing non-finite-loop rule handles them.

The result owns both vectors. The C++ adapter copies lengths and points into Qt containers before the `SelectionClipResult` is destroyed and retains no references or slices into it. An invalid result produces an empty `QList<QPolygonF>`.

`cpp/api/selection_api.{h,cpp}` gains the Qt adapters for clipping, marquee geometry, and the close-radius test. The existing `selection_bridge.h` remains the include-only membrane for generated declarations. `selection_geometry.cpp` includes only `api/selection_api.h` after the change and forwards all three shell functions to it. The new bridge verb is registered in `src/build_config/verb_manifest.tsv`. This preserves `pictura::clipSelectionShapeToCanvas` as the only clipping entry point and keeps generated-header ownership in the adapter layer.

Alternative considered: a count-prefixed `Vec<f64>`. Rejected because mixing integer counts and coordinates weakens validation. Alternative considered: a new bridge file. Rejected because the selection bridge already owns the neighboring marquee and radius adapters.

### D5. Keep both oracle layers

The six existing Qt cases keep their pinned assertions. Fast-path cases use exact ordered equality; filled-intersection cases also compare the bridged result with the complete Qt reference through the intersection-split boundary-segment helper.

A deterministic Qt differential corpus covers multiple loops, holes in both input orders, duplicate closing vertices, border-collinear edges, near touches, tangencies, and an open zero-area band. Rust unit tests cover the six inputs, invalid canvases, non-finite input, holes, exact-endpoint joining, near-miss separation, degenerate arity, malformed bridge payloads, open zero-area bands, and large-coordinate precision.

No new Qt Test suite or CMake source entry is needed. The existing bridge header and source lists already include the changed files.

## Risks / Trade-offs

- Point-touching contours may be split by the boolean library. The engine splices only exact shared arrangement endpoints; the pinned self-overlap and near-miss tests guard both outcomes.
- Fixed-point conversion can move intersections. The engine snaps near-border coordinates and clamps every returned vertex, while the `1e-6` parity gate covers non-border intersections. The large-coordinate test guards both rules.
- Invalid coordinates can panic an infallible dependency. Non-finite loops and invalid canvas sizes take the unchanged-input path and have unit tests.
- The dependency adds compile time and audit surface. It is confined to `pictura-select`, uses no default features, and replaces a substantially larger private topology implementation.
- Qt and the Rust library may simplify different collinear vertices or order loops differently. Boundary comparison removes harmless collinear vertices and matches segments without depending on loop order.

## Migration Plan

Add the engine module, dependency, and Rust tests first. Add the bridge and C++ adapter, then replace the production C++ boolean with delegation. The Qt oracle remains in the test file. No stored data or document format changes, so rollback is a source revert. The sequential phase runs the full build and focused tests before archive, then the aggregate gate.
