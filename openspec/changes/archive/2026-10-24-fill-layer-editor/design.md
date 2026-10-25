# Design: Fill layer editor

## Engine

### Gradient encoder

`encode_gradient_fill(kind, stops, angle)` is frozen and keeps writing only
custom stops. Add:

```rust
pub fn encode_gradient_fill_params(params: &GradientFillParams) -> AdjustmentData
```

It builds the same `GdFl` descriptor but writes `Rvrs` (bool) and `Scl `
(double) when they differ from the decode defaults (`false`, `100`). The
creation path and the editor use this encoder. `decode_adjustment` on its output
equals the input params.

### Pattern encoder

Add:

```rust
pub fn encode_pattern_fill(params: &PatternFillParams) -> AdjustmentData
```

The layout is the one already used by the render tests' `ptfl` helper
(`tests/mod.rs:193-232`): a `PtFl` descriptor with a `Ptrn` object carrying
`Idnt`, `Scl `, `Algn`, and an optional `phase` point. Promote it to `fill.rs`
and delete the test duplicate.

### Creation with full content

Add `create::add_fill_layer(doc, selection_path, spec, content)` where
`content` is an enum of solid colour / gradient params / pattern params. It
builds the layer, calls the new `insert_layer` (Change A), applies the default
mask through the caller, and returns the new path. The existing
`add_solid_fill`/`add_gradient_fill` wrappers keep their current signatures for
tests and non-dialog callers and delegate to it.

## Bridge

New `cxxqt_object/impl_layers/fill_content.rs`, registered in
`impl_layers/mod.rs` and declared in `cxxqt_object.rs` (or its own bridge
module, following `vector_masks.rs`):

- Reads: `layer_fill_kind(path)` -> `"solid-color" | "gradient" | "pattern" | ""`;
  `layer_fill_color(path)` -> packed ARGB; `layer_fill_gradient(path)` -> a
  tab-separated payload (kind, angle, scale, reverse, stops text);
  `layer_fill_pattern(path)` -> (pattern id, scale, link, origin).
- Writes: `set_layer_fill_color`, `set_layer_fill_gradient`,
  `set_layer_fill_pattern`. Each replaces the layer's `adjustment` block, is one
  undo state (label `"Solid Color Fill"`, `"Gradient Fill"`, `"Pattern Fill"`),
  and recomposites `layer.rect` (full when the rect is empty).
- Patterns: `document_pattern_rows(size)` -> `Vec<PatternRow { id, thumbnail }>`
  from `pictura_codec::decode_patterns(doc)`. The list is empty when the
  document has no `Patt`/`Pat2`/`Pat3` blocks.
- Creation: `add_fill_layer(kind, payload, spec)` reads the anchor from
  `layer_target`, inserts with the Change A helper, applies the fill default
  mask (Change A), selects the new layer, and records one state.

## UI

### Dialogs

- `SolidColorFillDialog` is the existing `ColorPickerDialog::getColor`; no new
  dialog file.
- `GradientFillDialog`:
  - A gradient sample (`GradientSwatch`) opens `GradientEditorDialog`.
  - Style combo: Linear, Radial, Angled, Reflected, Diamond.
  - Angle numeric `0..=360`, Scale numeric `1..=1000`, Reverse checkbox.
  - Dither and Align With Layer are present, checked/disabled, with the existing
    not-implemented tooltip (the renderer has neither).
  - `getFill(view, path, parent)` runs the modal and applies on OK.
- `PatternFillDialog`:
  - A grid/list of the document patterns; the current pattern is selected.
  - Scale numeric, Link With Layer checkbox, Snap To Origin button (resets the
    origin to `(0, 0)`).
  - Empty state: "No patterns are loaded — this document has no pattern
    resources." Pattern creation and this dialog are disabled when the list is
    empty.
- `fill_layer_editor.h/cpp` provides one dispatcher:
  `openFillLayerEditor(PictureView&, const QString& path, QWidget* parent)` that
  reads the kind and opens the matching editor, and
  `createFillLayer(...)` for the creation flow.

### Creation flow

- `Layer > New Fill Layer > Solid Color…` and `Gradient…`: show
  `LayerNewDialog` (prefilled name `Color Fill N` / `Gradient Fill N`), then the
  type editor; create once on acceptance; cancel at either step creates
  nothing.
- `Layer > New Fill Layer > Pattern…`: same flow, enabled only when the document
  has patterns.
- The Adjustments-panel fill icons (and the Layers-panel New Fill / Adjustment
  menu) create with the current defaults and then open the type editor; a
  cancelled editor leaves the default layer. This matches the quick-add role of
  those surfaces.
- Every successful creation is one undo state and selects the new layer.

### Dispatch

- `Layer > Layer Content Options` (`frame_menus_adjust.cpp`): if the current
  layer is a fill-content layer, open the fill editor; else show Properties.
- `layers_panel_events.cpp` double-click: if the click lands on a fill or
  adjustment content thumbnail, open the fill editor / Properties; a mask or
  vector-mask thumbnail keeps its existing consumed behavior; every other
  double-click keeps the Layer Style path.
- The row menu's `Edit Adjustment…` uses the same dispatcher.

### Pattern availability

`adjustments_panel.cpp` and the Layer menu enable the Pattern entry only when
`document_pattern_rows` is non-empty. The tooltip becomes the CS6 "No patterns
are loaded" reason rather than `— not implemented yet`; the latter remains for
features that are still absent.

## Test strategy

- Engine: gradient encode/decode round-trip with reverse and scale; pattern
  encode/decode round-trip; creation anchors and one undo state.
- Bridge: read/write for each kind; no-op write records nothing; unknown path
  refused; pattern list empty/populated.
- Qt: dialog values -> applied block; cancel leaves the layer unchanged;
  double-click and Content Options open the right dialog; creation flow creates
  once and selects; Pattern disabled without patterns.
- Existing `tst_layers_snapshot` gains fill-preview assertions; `tst_command_tree`
  gains the Pattern menu state.

## Risks

- `GradientFillParams` has no Dither or Align flag; the dialog shows both
  disabled so the UI matches CS6 without pretending the renderer honors them.
- Live on-canvas gradient/pattern dragging is not implemented; the dialog
  applies on OK. Marked as a `ponytail:` ceiling.
- Pattern names are not decoded from the PSD `Patt` blocks; the dialog shows the
  pattern id.
