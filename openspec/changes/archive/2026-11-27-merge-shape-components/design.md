## Context

[Adobe's path editing guide](https://helpx.adobe.com/ph_fil/photoshop/using/editing-paths.html#merge_overlapping_path_components) describes overlap mode and merging as separate operations. The former determines the result of the operands. Pictura applies the drawing settings when constructing the outline. The latter normalizes the resulting filled geometry.

## Goals / Non-Goals

Preserve holes, independent contours, and target identity. Record one history state. Component selection and path transforms remain separate features.

## Decisions

- Use the Path Selection options bar. Resolve exactly one target through the path bridge: an explicitly selected Paths row takes priority over shape-layer fallback; a selected vector-mask row targets that layer's mask. Only when the layer target is active without an explicit Paths-row target does the selected shape layer supply the outline. Select the active path as a whole, including when one of its contours has focus for direct manipulation.
- Use the existing polygon overlay engine with the target's effective fill rule. Work and saved paths use nonzero winding with implicit closure; flatten them as `path_coverage` does, using its 0.1 document-pixel adaptive tolerance and existing recursion cap. Layer vector masks use the renderer's effective fill rule and the closed `VectorSubpath` polyline points used by the renderer, already rounded to 1/256 pixel and subdivided at 16 steps per cubic. Open mask contours do not contribute to the overlay but are retained unchanged in the result.
- Identify connected interaction groups including containing and enclosed contours, so ambient winding or parity contributes to each overlay. Resolve the effective filled region for the whole group, not isolated operands, and emit outer and hole contours with consistent opposite winding. Retain exactly the original anchors, handles, closure and order of disjoint contours outside the affected groups. Do not reapply the global drawing operation.
- Overlay converts floating-point geometry to an integer grid; its quantization and the mask's encoding can shift output vertices. Relative to the target's effective input polylines, accept at most 1/256 document pixel boundary displacement in either direction, with unchanged fill topology; decline the merge if that bound cannot be met. For Work and saved paths, the comparison baseline is the 0.1-pixel flattened path, not the mathematical Bézier; the existing flattening recursion cap applies. Resolve source and candidate fills independently with a shared i64 overlay grid; compare component and hole counts, new edge contacts, and certify boundary distance in both directions by bounded adaptive edge subdivision. Decline on comparison-budget exhaustion. This comparison is conservative and depends on overlay rounding and floating-point contact predicates; it does not prove arbitrary sub-grid topology or nesting hierarchy.
- Prepare the complete result before mutating the document. Keep mask flags and style through the existing path edit route. If the normalized mask would lose its last filled contour and become permissive under current mask semantics, keep the original. Decline imported layer masks whose stored operations cannot safely survive the existing editor's rewrite; preserve their original bytes and history.
- Bound contour and vertex counts before invoking the overlay. A declined or unchanged merge creates no history, preserves the path and leaves any redo branch intact.

## Risks / Trade-offs

Normalized curves may have more anchors. Pictura's fill and selection geometry are the comparison target. Photoshop's specific curve fitting and imported operand semantics require separate evidence.
