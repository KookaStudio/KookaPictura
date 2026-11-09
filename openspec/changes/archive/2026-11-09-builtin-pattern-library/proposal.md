# Built-in pattern library for Pattern fill

## Why

`Layer > New Fill Layer > Pattern…` is disabled on a document that carries no
`Patt`/`Pat2`/`Pat3` resource, so a blank document cannot create a pattern fill
at all. Photoshop always offers its bundled patterns. The app should provide a
small, project-authored built-in library so Pattern fill is available on every
open document, without committing Adobe preset patterns.

A built-in pattern must be written into the document before the fill layer
references it: the compositor, the Layers thumbnail, and the saved PSD all
resolve pattern ids through the document's pattern library.

## What changes

- `pictura-codec` exposes `builtin_patterns()`, a small opaque RGB pattern
  library authored by the project.
- The app lists the document's decoded patterns first, then the built-ins whose
  ids are not already present.
- `Layer > New Fill Layer > Pattern…`, the Layers-panel New Fill/Adjustment
  menu, and the Adjustments-panel Pattern icon are enabled whenever a document
  is open.
- Creating or editing a pattern fill with a built-in id embeds that pattern into
  the document's `Patt` block in the same edit, so the fill renders and the
  saved file carries it.
- The Pattern Fill editor shows the built-ins on a document with no embedded
  patterns; its empty state remains only as a defensive fallback.
- Tests cover the library, the embed-on-create path, and the enabled menu/panel
  entries on a blank document.

## Capabilities

- `imaging/adjustment-layer-rendering` — a new requirement for the built-in
  pattern library and embedding.
- `ui/fill-layer-editor` — the Pattern editor lists built-ins and embeds the
  chosen one.
- `ui/layers-panel` — Pattern is available on any open document.
- `ui/adjustments-panel` — the Pattern icon and grid entry are enabled on any
  open document.

## Impact

- `crates/pictura-codec/src/patterns.rs` and its exports.
- `crates/pictura-app/src/cxxqt_object/impl_layers/fill_content/*` — available
  ids, previews, and the embed-on-create/edit helper.
- `crates/pictura-app/cpp/frame_menus.cpp`, `panels/layers_panel.cpp`,
  `panels/adjustments_panel.cpp`, `pattern_fill_dialog.cpp`.
- Qt tests in `tst_fill_layer_editor` and `tst_adjustments_panel`.
