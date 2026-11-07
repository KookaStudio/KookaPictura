# Proposal

## Why

The Paint Mask button currently toggles only its own checked state, so users cannot refine a selection by painting. Issue #172 completes that control using the existing selection, mask-painting, presentation, and history infrastructure, with entry and exit behavior clarified by the user on 2026-11-07.

## What Changes

- Make the toolbar button and `Q` enter and leave a document-local Quick Mask mode, with a temporary Channels row and an editable grayscale coverage mask.
- Enter without a pixel selection with a clear overlay; black paint adds red protection. Enter with a selection with clear selected areas and red protected exterior. Without a selected layer, show a blocked cursor and reject painting.
- On exit, make white/clear mask areas selected and black/red areas unselected, preserving every intermediate coverage value. An untouched round trip preserves the distinction between no selection and an explicit mask.
- Highlight the active layer's row red while Quick Mask is active, restoring its ordinary selected-row highlight on exit without changing the layer's color label.
- Route Brush, Pencil, Eraser, existing grayscale-compatible fill tools, and supported mask filters to coverage instead of layer pixels, preserving blend-mode behavior in each display polarity. Reject unsupported tools and content-move/duplicate gestures and previews rather than falling through to pixel or layer-mask editing.
- Add the default red 50% overlay, composite/grayscale view toggle (`~`), polarity switch (`Alt`-click), and double-click options for display polarity, color, and opacity. Keep presentation settings in application preferences.
- Keep Quick Mask's black/white paint pair separate from remembered image and layer-mask colors; exit and document activation restore the pair belonging to the resulting edit target.
- Record committed mask edits through selection history; keep display controls and entry/exit free of extra history states, following the tool document's proposed behavior without asserting verified CS6 history parity.
- Keep the mode, overlay, and temporary channel out of saved files. Persist coverage only through the existing Save Selection alpha-channel path.

## Capabilities

### New Capabilities

- `tools/quick-mask`: selection-mask entry/exit, edit routing and blocking, display controls, history, lifecycle, and persistence boundaries.

### Modified Capabilities

- `ui/workspace-shell-polish`: replace the explicit Paint Mask stub requirement with a document-bound working mode control.
- `ui/channels-panel`: add the temporary Quick Mask row, coverage thumbnail, and safe interaction with ordinary channel controls.
- `ui/layers-panel`: add the active layer's transient red Quick Mask highlight.

## Impact

- Rust: `pictura-select` coverage helpers; `pictura-app` document session, paint/fill/filter routing, selection history, presentation and cxx-qt declarations. Keep runtime Quick Mask state outside `pictura_core::Document` and `Document.channels`.
- Qt: toolbox, command/shortcut routing, tool cursor/admission, canvas presentation, ColorState, Channels panel, options dialog, session preference storage, and their API bridges.
- Tests: a dedicated offscreen `tst_quick_mask` suite, selection-engine checks, and app state/save checks; scenario-to-test mapping in `test-plan.md`.
- Build registration: explicitly register any added C++ source/header and Qt Test executable. No new dependencies, Adobe assets, or binary fixture baselines are proposed.
- Delivery: update the two Quick Mask entries in `ROADMAP.md` and archive this change in the shipping PR. The long-form `docs/` corpus remains read-only in this task; its unresolved CS6 questions are recorded in the design instead.
