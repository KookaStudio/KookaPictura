# Design

## Context

The current registry creates rectangle and ellipse handlers in `tools.cpp`; their options and classification are spread across the catalog, options bar, controller and `tools_selection_move.cpp`. The selection pipeline already supports rectangular masks, feather and combining. `tools_selection_move.cpp` starts selection movement on press inside a selection, whereas these tools must distinguish a click from a move drag.

## Goals / Non-Goals

**Goals:** Reuse selection and history infrastructure and keep single-line interactions compatible with selection move and menus.

**Non-Goals:** Change the existing rectangle/ellipse interaction, establish an Adobe-exact feather kernel, or add default shortcuts to the two new tools.

## Decisions

- Register distinct tool IDs and flyout entries, separating flyout/Alt-click navigation from keyboard shortcut cycling. Including the new tools in the M cycle would conflict with the documented default. From either unassigned line tool, the next M-cycle gesture selects Rectangular Marquee; the same fallback applies to plain M when Shift is required.
- Form a document-clipped one-pixel rectangle from the clicked image pixel, then use the existing selection commit and feather path. A separate mask rasterizer would duplicate selection and undo behavior.
- Include the IDs in the selection-tool and selection-menu classifications, cursor handling and options routing. Defer the click-versus-move decision for a press inside a selection until release or a genuine drag, so a click creates a band and dragging still moves the selection. Apply the existing combine-modifier rules before selection movement.
- Reuse the shared selection options model for mode and feather while hiding Style and Anti-alias for these IDs. Keep rectangle and ellipse state intact. Preserve the existing feather approximation rather than claiming pixel parity. Offer Refine Edge with the same availability and action as the other marquee tools.

## Risks / Trade-offs

- [Click and selection movement share a press] → Cover both cases for both tools and test that a click is not swallowed by movement handling.
- [Thin feathered bands can fall below the visible contour threshold] → Assert coverage values, not just marching ants; after a nonempty result has no pixel above 50% coverage, show the documented warning while retaining the selection and its history. The warning is not currently emitted by the marquee commit path. Clamp-to-edge feathering keeps a full-document band fully selected.
- [Flyout cycling and shortcut cycling differ] → Test both paths and default shortcut assignments.

## Migration Plan

No data migration is required. Roll back by removing the tool registrations and UI entries with the implementation.
