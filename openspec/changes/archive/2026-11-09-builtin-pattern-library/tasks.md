# Tasks

## 1. Tests first

- [x] 1.1 `pictura-codec::builtin_patterns_are_available` — the library is non-empty, deterministic, and its ids are unique.
- [x] 1.2 `pictura-codec::builtin_patterns_encode_into_a_document` — a built-in encodes and appends to a document's `Patt` block, then decodes back.
- [x] 1.3 `tst_fill_layer_editor::patternBuiltinsOnBlankDocument` — a blank document lists the built-ins and the dialog is not empty.
- [x] 1.4 `tst_fill_layer_editor::patternBuiltinFillEmbedsAndRenders` — creating a fill with a built-in embeds it and the row thumbnail is the tiled pattern.
- [x] 1.5 `tst_fill_layer_editor::layerFillMenuPatternGating` — the Layer menu Pattern action is enabled on a blank document.
- [x] 1.6 `tst_fill_layer_editor::patternIconGating` — the Adjustments-panel Pattern icon is enabled on a blank document.
- [x] 1.7 `tst_adjustments_panel::patternIconCreatesDefaultFill` — clicking the Pattern icon creates a default fill and opens the editor.
- [x] 1.8 `tst_adjustments_panel::documentEnablesGrid` — every grid entry, including Pattern, is enabled with a document.
- [x] 1.9 `tst_adjustments_panel::layersPanelAdjustmentMenuFollowsCatalogue` — the Layers-panel Pattern entry is enabled.

## 2. Built-in library

- [x] 2.1 Add `builtin_patterns()` to `pictura-codec`, backed by a `OnceLock`, with two small opaque patterns.
- [x] 2.2 Export it from `pictura-codec` and cover it with unit tests.

## 3. App bridge

- [x] 3.1 Return document patterns first, then built-ins whose ids are not already present, from the available-pattern query and thumbnail lookup.
- [x] 3.2 Add the embed helper: encode a built-in and append it to the document's `Patt` block when a fill references it.
- [x] 3.3 Embed inside the `add_fill_layer` and `set_layer_fill_pattern` edits so the pattern and the layer are one history state.

## 4. UI

- [x] 4.1 Enable `Layer > New Fill Layer > Pattern…` whenever a document is open.
- [x] 4.2 Enable the Layers-panel and Adjustments-panel Pattern entries whenever a document is open; drop the no-patterns reason.
- [x] 4.3 Keep the Pattern Fill editor's empty state only as a defensive fallback; list the built-ins on a blank document.

## 5. Finish

- [x] 5.1 Run `cargo nextest run -p pictura-codec -p pictura_app`, the two Qt suites, and `scripts/check-test-plan.py`.
- [x] 5.2 Run `scripts/verify-fast.sh`.
- [x] 5.3 Archive the change and update PR #1261.
