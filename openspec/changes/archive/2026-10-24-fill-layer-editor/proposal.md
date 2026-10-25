# Proposal: Fill layer editor

## Why

Fill layers are created with fixed defaults and cannot be edited: Solid Color
uses the foreground colour with no picker, Gradient is always black-to-white,
Pattern creation is inert, and `Layer > Layer Content Options` only shows the
Properties panel's read-only note. CS6 opens the fill type's editor when the
layer is created and when its content thumbnail is double-clicked
(`docs/05-layers/fill-layers.md:37-49`). #895 requires the created layer to open
its editor, and the user asked for real editors rather than a Properties note.

## What Changes

- **Solid Color** — the Color Picker edits the fill's RGB and applies on OK.
- **Gradient** — a Gradient Fill dialog reuses the existing Gradient Editor and
  exposes Style, Angle, Scale, and Reverse; Dither and Align With Layer are
  present but disabled with the not-implemented tooltip.
- **Pattern** — a Pattern Fill dialog lists the patterns embedded in the open
  document with Scale, Link With Layer, and Snap To Origin. Pattern creation and
  editing are unavailable when the document has no patterns; the preset library
  and `Edit > Define Pattern` remain #262/#599.
- **Creation** — `Layer > New Fill Layer` runs the New Layer dialog then the
  type editor and creates once; the Adjustments-panel fill icons create with
  defaults and open the type editor. The created layer is anchored above the
  active layer and selected (Change A).
- **Dispatch** — `Layer > Layer Content Options` and a double-click on a fill
  layer's content thumbnail open the type editor; adjustment layers keep the
  Properties panel.
- **Engine** — add a full-parameter gradient encoder that writes `Rvrs`/`Scl `
  and a public `PtFl` encoder, so editor round-trips keep reverse and scale.

## Capabilities

### New Capabilities

- `ui/fill-layer-editor`: the Solid Color, Gradient, and Pattern fill editors,
  their creation flow, and their dispatch from the Layers panel.

### Modified Capabilities

- `ui/layers-panel`: the Fill / Adjustment creation menu enables Pattern when
  the document has patterns, Layer Content Options opens the type editor for a
  fill layer, and a content-thumbnail double-click opens that editor.
- `ui/adjustments-panel`: the Pattern fill icon is enabled when the document
  has patterns.
- `imaging/adjustment-layer-rendering`: adds the full-parameter `GdFl` encoder
  and the `PtFl` encoder.

## Impact

- Engine: `crates/pictura-render/src/fill.rs`, `document_ops/layer_ops/create.rs`,
  tests.
- Bridge: `crates/pictura-app/src/cxxqt_object/impl_layers/fill_content.rs`
  (new), `impl_layers_rasterize.rs`, `cxxqt_object.rs`.
- UI: new `solid_color_fill_dialog.*`, `gradient_fill_dialog.*`,
  `pattern_fill_dialog.*`, `fill_layer_editor.*`; changes to
  `frame_menus.cpp`, `frame_menus_adjust.cpp`, `frame_build.cpp`,
  `layers_panel_events.cpp`, `layers_panel_menu.cpp`, `adjustments_panel.cpp`;
  `CMakeLists.txt` source entries.
- Tests: new `tst_fill_layer_editor.cpp`; extensions to
  `tst_layers_panel`, `tst_layers_snapshot`, `tst_command_tree`, and
  `pictura-render` tests.
- Docs: `ROADMAP.md` pattern-fill entry; `docs/` already contracts the dialogs.
- Related: #895 closed by this change plus the bundle; #262/#599 retain the
  preset library and Define Pattern.
